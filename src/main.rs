mod player;
mod lyrics;
mod radio;
mod ui;
mod utils;


fn main() {
    println!("Starting OrkaPlayer Linux v2.3.0...");

    if let Err(e) = ui::run_ui() {
        eprintln!("UI Error : {}", e);
    }
}
