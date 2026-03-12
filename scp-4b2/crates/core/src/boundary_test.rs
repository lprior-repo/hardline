use crate::domain::identifiers::WorkspaceName;
pub fn test() {
    let s = "あ".repeat(100);
    let result = WorkspaceName::parse(s);
    println!("{:?}", result);
}
