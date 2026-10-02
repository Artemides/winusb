use std::io::{self, Read, Seek, SeekFrom};

pub struct ExtentReader<R> {
    source: R,
    extents: Vec<Extent>,
    len: u64,
    position: u64,
}

pub struct Extent {
    pub source_offset: u64,
    pub len: u64,
}

impl<R: Read + Seek> ExtentReader<R> {
    pub fn new(source: R, extents: Vec<Extent>) -> io::Result<Self> {
        if extents.iter().any(|e| e.len == 0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "zero len extents",
            ));
        }

        let len = extents
            .iter()
            .try_fold(0_u64, |total, e| total.checked_add(e.len))
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "virtual file too large"))?;

        Ok(Self {
            source,
            extents,
            len,
            position: 0,
        })
    }

    fn locate(&self, position: u64) -> Option<(&Extent, u64)> {
        let mut start = 0_u64;

        for extent in &self.extents {
            let end = start + extent.len;

            if position < end {
                let offset = position - start;

                return Some((extent, offset));
            }

            start = end;
        }

        None
    }
}

impl<R: Read + Seek> Read for ExtentReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() || self.len == self.position {
            return Ok(0);
        }

        let (extent, offset) = self.locate(self.position).expect("valid position");
        let available = extent.len - offset;
        let wanted = available.min(buf.len() as u64) as usize;

        self.source
            .seek(SeekFrom::Start(extent.source_offset + offset))?;
        let bytes = self.source.read(&mut buf[..wanted])?;

        self.position += bytes as u64;

        Ok(bytes)
    }
}

impl<R: Read + Seek> Seek for ExtentReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let next = match pos {
            SeekFrom::Start(pos) => pos as i128,
            SeekFrom::Current(delta) => self.position as i128 + delta as i128,
            SeekFrom::End(delta) => self.len as i128 + delta as i128,
        };

        if next < 0 || next > self.len as i128 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "next position out of vfile",
            ));
        }

        self.position = next as u64;

        Ok(self.position)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn seeks_and_reads() -> io::Result<()> {
        let source = Cursor::new(b"abc__defgh__ijk:".to_vec());

        let mut v_file = ExtentReader::new(
            source,
            vec![
                Extent {
                    len: 3,
                    source_offset: 0,
                },
                Extent {
                    len: 5,
                    source_offset: 5,
                },
                Extent {
                    len: 3,
                    source_offset: 12,
                },
            ],
        )?;

        let mut all = String::new();
        v_file.read_to_string(&mut all)?;
        assert_eq!(all, "abcdefghijk");

        v_file.seek(SeekFrom::Start(6))?;
        let mut bytes = [0_u8; 3];

        v_file.read_exact(&mut bytes)?;
        assert_eq!(&bytes, b"ghi");

        Ok(())
    }
}
