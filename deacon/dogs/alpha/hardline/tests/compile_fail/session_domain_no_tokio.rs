// This should fail to compile - domain cannot import tokio
use tokio::sync::Mutex;

fn main() {}
