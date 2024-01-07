use std::{
    fs::{self},
    process::{self, Command, Stdio},
    sync::{
        mpsc::{channel, Sender},
        Arc, Mutex, RwLock,
    },
    thread::{sleep, spawn},
    time::{Duration, Instant},
};

use anyhow::Result;
use clap::Parser;
use tempfile::{tempdir_in, TempDir};
use tracing::{error, info};

/// Program used to brute force a SSH public key with certain contents
#[derive(Parser, Debug)]
#[command(author = "KuNet", version = env!("CARGO_PKG_VERSION"))]
struct Args {
    /// Comment (in most cases your email address)
    #[arg(short = 'C', long)]
    comment: String,

    /// What to search for separated by commas
    #[arg(short, long)]
    search: String,

    /// Key type
    #[arg(
        short = 't',
        long = "type",
        value_name = "TYPE",
        default_value = "ed25519"
    )]
    key_type: String,

    /// Print the progress every ? times
    #[arg(long, default_value_t = 100)]
    print_every: u64,

    /// Output name
    #[arg(short, long, default_value = "bruted")]
    output: String,
}

struct State {
    counter: u64,
    start: Instant,
    iteration: Instant,
}

impl State {
    fn print_details(&mut self) {
        let total_duration = Instant::now().duration_since(self.start);
        let iteration_duration = Instant::now().duration_since(self.iteration);
        self.iteration = Instant::now();

        info!(
            "{:#?} total (last {:#?}); {} attempts",
            total_duration, iteration_duration, self.counter
        );
    }
}

fn guess(
    args: Arc<Args>,
    search_terms: Arc<Vec<String>>,
    path: Arc<TempDir>,
    state: Arc<Mutex<State>>,
    finished: Arc<RwLock<bool>>,
    done: Sender<usize>,
    number: usize,
) {
    let pub_path = path.path().join(number.to_string() + ".pub");
    let private_path = path.path().join(number.to_string());

    loop {
        if *finished.read().expect("could not read finished state") {
            return;
        }

        let mut command = Command::new("ssh-keygen");
        command.current_dir(path.path());
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());
        command.arg("-t");
        command.arg(&args.key_type);
        command.arg("-C");
        command.arg(&args.comment);
        command.arg("-f");
        command.arg(number.to_string());
        command.arg("-N");
        command.arg("\"\"");

        command.status().expect("generating key failed");

        let content = fs::read_to_string(pub_path.clone()).expect("could not read key pub data");
        let split: Vec<&str> = content.split(' ').collect();
        if split.len() != 3 {
            panic!("key does not have 3 parts how");
        }

        let word = split
            .get(1)
            .expect("this can't happen")
            .to_ascii_lowercase();

        for term in search_terms.iter() {
            if word.contains(term) {
                let _ = done.send(number);
                *finished.write().expect("could not write to finished state") = true;
                return;
            }
        }

        fs::remove_file(pub_path.clone()).expect("could not delete public key");
        fs::remove_file(private_path.clone()).expect("could not delete private key");

        {
            let mut s = state.lock().expect("could not get state");
            s.counter += 1;
            if s.counter % args.print_every == 0 {
                s.print_details();
            }
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let args_arc = Arc::new(args);
    tracing_subscriber::fmt::init();

    let split = args_arc.search.split(',');
    if split.clone().count() == 0 {
        error!("search for something! try something like \"-s real,word,search\"");
        process::exit(1);
    }

    if split
        .clone()
        .any(|item| !item.is_ascii() && item.chars().any(|c| !char::is_alphanumeric(c)))
    {
        error!("make sure your search terms are alphanumeric");
        process::exit(1);
    }

    let search_terms: Vec<String> = split.map(|s| s.to_ascii_lowercase()).collect();
    let search_terms_arc = Arc::new(search_terms);

    info!("searching for:");
    for search_term in search_terms_arc.iter() {
        info!(" - {}", search_term);
    }

    let temp = tempdir_in(".")?;
    info!(
        "created temporary directory {:?} (delete me if you cancel!)",
        temp.path().file_name().expect("temp directory has no name")
    );

    let path_arc = Arc::new(temp);

    let threads = num_cpus::get();
    info!("starting {} threads", threads);

    let state = Arc::new(Mutex::new(State {
        counter: 0,
        start: Instant::now(),
        iteration: Instant::now(),
    }));

    let (sender, receiver) = channel::<usize>();
    let finished = Arc::new(RwLock::new(false));

    for n in 0..threads {
        let args_clone = args_arc.clone();
        let search_terms_clone = search_terms_arc.clone();
        let path_clone = path_arc.clone();
        let state_clone = state.clone();
        let finished_clone = finished.clone();
        let sender_clone = sender.clone();

        spawn(move || {
            guess(
                args_clone,
                search_terms_clone,
                path_clone,
                state_clone,
                finished_clone,
                sender_clone,
                n,
            )
        });
    }

    let worker = receiver.recv().unwrap();
    state.lock().unwrap().print_details();
    info!("found!");

    let pub_path = path_arc.path().join(worker.to_string() + ".pub");
    let private_path = path_arc.path().join(worker.to_string());

    // just in case copies break FOR SOME REASON copy the private FIRST
    fs::copy(private_path, &args_arc.output)?;
    fs::copy(pub_path, args_arc.output.to_owned() + ".pub")?;

    // ok yeah I know this is really bad memory management
    // I should have just written this in Zig
    sleep(Duration::from_secs(1));

    Ok(())
}
