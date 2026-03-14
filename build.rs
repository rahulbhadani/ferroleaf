fn main() {
    // Tell cargo to re-run if fonts change
    println!("cargo:rerun-if-changed=assets/");
}
