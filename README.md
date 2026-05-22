A monorepo for applications powered by the **Crunchie** math engine.

## Crunchie-pad

![Crunchie Pad](./docs/Crunchie-pad.png)

["Proof of life" application for the core](./crunchie-pad): It's like sticky notes, but with math.
- Just write your math, with or without units, like a normal person
- Hitting `tab` will apply gray "autocalculations"

## Install

Dependencies:
- [rust installed in your system](https://rust-lang.org/tools/install/)

Build & install with cargo:
```bash
cargo install --git https://github.com/Taugeshtu/crunchie-apps crunchie-pad
```

_Alternatively:_
```bash
# navigate to where you want it to live, for example, ~/Applications/Gits
git clone https://github.com/Taugeshtu/crunchie-apps
cd crunchie-apps
cargo install --path crunchie-pad --root ~/.local
```
