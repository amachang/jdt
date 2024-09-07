fn main() {
    let dir = std::env::args().nth(1).expect("no dir given");
    jdt::walk_dir(&dir, |path| {
        println!("{}", path.display());
        ()
    });
}
