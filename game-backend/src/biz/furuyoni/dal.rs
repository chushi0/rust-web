use super::data::{Character, SpecialEffect};
use anyhow::Result;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;

pub async fn get_random_characters(rng: &mut ChaCha8Rng, c: usize) -> Result<Vec<Character>> {
    let mut conn = web_db::create_connection(web_db::RDS::Furuyoni).await?;
    let mut tx = web_db::begin_tx(&mut conn).await?;

    let mut characters = web_db::furuyoni::get_all_characters(&mut tx).await?;
    characters.shuffle(rng);
    characters.resize_with(c, || panic!("should not call this"));

    let characters: Vec<Character> = characters
        .iter()
        .map(|character| Character {
            id: character.rowid,
            primary_attack_distance: vec![],
            secondary_attack_distance: vec![],
            special_effect: SpecialEffect {},
            status: vec![],
        })
        .collect();
    Ok(characters)
}
