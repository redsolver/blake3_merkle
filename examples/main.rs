use blake3_merkle::Merkle;
use std::{error::Error, fs::File, io::copy};

fn main() -> Result<(), Box<dyn Error>> {
  let fpath = "/Users/z/Downloads/1.pdf";

  let mut blake3 = blake3::Hasher::new();
  copy(&mut File::open(&fpath)?, &mut blake3)?;

  let mut merkle = Merkle::new();
  copy(&mut File::open(&fpath)?, &mut merkle)?;
  dbg!(&merkle.li);
  dbg!(merkle.blake3());
  dbg!(blake3.finalize());

  Ok(())
}
