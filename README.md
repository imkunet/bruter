# bruter
brute force an ssh key

let's say you want an ssh public key that has the word `book` OR `worm` somewhere in it...
you came to the right place.

```
bruter -C myemail@gmail.com -s "book,worm"
```
*this executed on my computer with an `AMD 5800X3D` in 94.679234589 seconds*

ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAINqII`boOk`CVnuv1p+G+KnvMj4IlD14kGdZoGkjIu+17K myemail@gmail.com

congratulations. also fair warning: **good luck getting anything above 4 characters**. *you've been warned :)*!

## how to run (lazy edition)

```
cargo run -- -C "im@kunet.dev" -s "blob,tree"
```

## strategy

personally if you want to seem cool, I recommend choosing a lot of 5-6 character
words to brute force and add them to the search list.

- beans
- kotlin
- drone

really, the possibilities are kinda endless
