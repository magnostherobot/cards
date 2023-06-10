use std::error::Error;
use log::error;

use cards::run;

fn main() -> Result<(), Box<dyn Error>> {
    pollster::block_on(run())
}

#[cfg_attr(target_arch="wasm32", wasm_bindgen(start))]
pub async fn wasm_main() {
    match run().await {
        Ok(_) => (),
        Err(e) => error!("{e:?}"),
    }
}
