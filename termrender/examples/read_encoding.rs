fn main() {
    let stdin = std::io::stdin();
    let mut message = String::new();
    while let Ok(size) = stdin.read_line(&mut message) {
        if size != 0 {
            if message == "q" || message == "Q" {
                return;
            }
            println!("{}", message.trim());
            message.clear();
        }
    }
}
