fn main() {
    println!("cargo:rerun-if-changed=queries");
    println!("cargo:rerun-if-changed=schema");
}
