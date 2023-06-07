use cards::run;

fn main() {
    let _ = pollster::block_on(run());
}

