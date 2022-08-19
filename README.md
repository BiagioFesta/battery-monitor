# battery-monitor
Simple service for monitoring battery and sending notifications when capacity is lower than a threshold.

Typically, Linux Desktop Environments are shipped with an integrated system for battery monitoring. 
However, this minimal service come handy for those environments lacking of such functionality (for instance, [i3wm](https://i3wm.org/)).

## Installation

### Requirements
* [Rust toolchain](https://rustup.rs/)

### Compilation
* Clone and checkout the repository:
```
git clone https://github.com/BiagioFesta/battery-monitor.git && cd battery-monitor 
```

* Compile it:
```
cargo build --release
```

* Run the binary:
```
./target/release/battery-monitor
```

*The binary is standalone. You can copy it in a suitable directory and start it in the most convenient way for your environment.*
