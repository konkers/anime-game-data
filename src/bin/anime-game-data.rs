use anime_game_data::AnimeGameData;

#[tokio::main]
async fn main() {
    let mut data = AnimeGameData::new().unwrap();

    data.update().await.unwrap();
    //println!("{:?}", data);
}
