use file_processing::mail;

fn main() {
    println!("{}", mail::text().unwrap_or("Failed to get text".to_string()));
    println!("{}", mail::metadata().unwrap_or("Failed to get metadata".to_string()));
    println!("{}", mail::pdf().unwrap_or("Failed to get pdf".to_string()));
}
