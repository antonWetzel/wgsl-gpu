include!(concat!(env!("OUT_DIR"), "/shaders.rs"));

fn main() {
    println!("Shaders: {}", SHADERS);
}
