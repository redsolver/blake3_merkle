#![feature(new_uninit)]

use blake3::{
  guts::{parent_cv, ChunkState, CHUNK_LEN},
  Hash,
};

use std::{
  io::{Error, Write},
  mem::replace,
};

#[derive(Debug)]
pub struct HashDepth {
  hash: Hash,
  depth: u8,
}

// (1<<10) * 1024 = 1MB
pub const BLOCK_CHUNK: u8 = 10;

#[derive(Debug)]
pub struct Merkle {
  pub li: Vec<HashDepth>,
  pub pos: usize,
  pub n: u64,
  pub state: ChunkState,
}

impl Merkle {
  pub fn new() -> Self {
    Merkle {
      li: vec![],
      pos: 0,
      n: 0,
      state: ChunkState::new(0),
    }
  }

  pub fn finalize(&mut self) {
    let mut len = self.li.len();
    let end = len == 0;
    if self.pos != 0 {
      self.push(true);
      len = self.li.len();
    } else if end {
      return;
    }

    let li = &mut self.li;
    len -= 1;
    let mut hash = li[len].hash;

    while len > 0 {
      len -= 1;
      let left = &li[len];
      if left.depth == BLOCK_CHUNK {
        len += 1;
        li[len].hash = hash;
        li.truncate(len + 1);
        break;
      }
      hash = parent_cv(&left.hash, &hash, 0 == len);
    }
    //dbg!(&li);
  }

  pub fn blake3(&self) -> Hash {
    let li = &self.li;
    let len = li.len();
    match len {
      0 => ChunkState::new(0).update(&[]).finalize(true),
      1 => li[0].hash,
      2 => parent_cv(&li[0].hash, &li[1].hash, true),
      len => {
        let mut hash_len = len / 2;
        let end = len % 2;
        let mut box_len = hash_len + end;
        let mut hash_li = unsafe { Box::<[Hash]>::new_uninit_slice(box_len).assume_init() };
        if end != 0 {
          hash_li[hash_len] = li[len - 1].hash;
        }

        while hash_len != 0 {
          hash_len -= 1;
          let t = hash_len * 2;
          hash_li[hash_len] = parent_cv(&li[t].hash, &li[t + 1].hash, false);
        }
        while box_len > 2 {
          let mut hash_len = box_len / 2;
          let end = box_len % 2;
          let len = hash_len + end;
          let mut li = unsafe { Box::<[Hash]>::new_uninit_slice(len).assume_init() };
          if end != 0 {
            li[hash_len] = hash_li[box_len - 1];
          }
          while hash_len != 0 {
            hash_len -= 1;
            let t = hash_len * 2;
            li[hash_len] = parent_cv(&hash_li[t], &hash_li[t + 1], false);
          }
          box_len = len;
          hash_li = li
        }

        parent_cv(&hash_li[0], &hash_li[1], true)
      }
    }
  }

  fn push(&mut self, finalize: bool) {
    let li = &mut self.li;
    let mut len = li.len();
    let mut hash = self.state.finalize(finalize && len == 0);

    let mut depth = 0;
    while len > 0 {
      len -= 1;
      let left = &li[len];
      if left.depth == depth {
        depth += 1;
        hash = parent_cv(&left.hash, &hash, finalize && len == 0);
        li.pop();
        if depth == BLOCK_CHUNK {
          break;
        }
      }
    }
    li.push(HashDepth { depth, hash });
  }
}

impl Write for Merkle {
  fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
    let len = buf.len();
    let mut pos = self.pos;
    let mut n = self.n;
    let mut remain = CHUNK_LEN - pos;
    let mut begin = 0;

    while begin < len {
      if remain == 0 {
        self.push(false);
        n += 1;
        self.state = ChunkState::new(n);
        pos = 0;
        remain = CHUNK_LEN;
      }
      let diff = len - begin;
      if diff < remain {
        pos += diff;
        self.state.update(&buf[begin..]);
        break;
      } else {
        let end = begin + remain;
        self.state.update(&buf[begin..end]);
        begin = end;
        remain = 0;
        pos = CHUNK_LEN;
      }
    }
    self.pos = pos;
    self.n = n;
    Ok(len)
  }

  fn flush(&mut self) -> Result<(), Error> {
    Ok(())
  }
}
