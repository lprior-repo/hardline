use cucumber::World;

#[derive(Debug, Default, World)]
pub struct HardlineWorld {}

#[tokio::main]
async fn main() {
    HardlineWorld::run("../features").await;
}
