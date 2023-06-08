use std::error::Error;

use cards::run;

fn main() -> Result<(), Box<dyn Error>> {
    pollster::block_on(run())
}
