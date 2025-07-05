# Rush - a rust based terminal

A simple terminal based vim/neovim like text editor

## Dependencies
- rust

## Instalation

### Arch Based Distros

``` bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

sudo pacman -S git

git clone https://github.com/Edgarmls1/rush.git
cd rush
makepkg -si
```

### Other Linux Distros and Mac

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

sudo apt install git # for debian based
sudo dnf install git # for redhat based
brew install git # for macos

git clone https://github.com/Edgarmls1/rush.git
cd rush
cargo build --release
sudo cp target/release/rush /usr/bin/
```

### Windows

```bash
winget install -e --id Git.Git

git clone https://github.com/Edgarmls1/rush.git
cd ReEdit
cargo build --release
cargo install --path .
```
