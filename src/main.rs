mod aps;
mod chat;
mod course_info;
mod loader;
mod rag;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("terminal") {
        chat::run_terminal_chat();
        return;
    }

    println!("CHATBOX_CPUT_PROSPECTUS");
    println!("Document-grounded RAG assistant for the official CPUT prospectus.\n");
    chat::run_chatbox();
}
