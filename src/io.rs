use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

pub type IoResult<T> = Result<T, Box<dyn std::error::Error>>;

// ============================================================
// Vec<Vec<usize>>
// ============================================================

pub fn write_vv_usize<P: AsRef<Path>>(path: P, data: &Vec<Vec<usize>>) -> IoResult<()> {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    bincode::serialize_into(writer, data)?;
    Ok(())
}

pub fn read_vv_usize<P: AsRef<Path>>(path: P) -> IoResult<Vec<Vec<usize>>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let data = bincode::deserialize_from(reader)?;
    Ok(data)
}

// ============================================================
// Vec<f64>
// ============================================================

pub fn write_v_f64<P: AsRef<Path>>(path: P, data: &Vec<f64>) -> IoResult<()> {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    bincode::serialize_into(writer, data)?;
    Ok(())
}

pub fn read_v_f64<P: AsRef<Path>>(path: P) -> IoResult<Vec<f64>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let data = bincode::deserialize_from(reader)?;
    Ok(data)
}

// ============================================================
// Vec<u64>
// ============================================================

#[allow(dead_code)]
pub fn write_v_u64<P: AsRef<Path>>(path: P, data: &Vec<u64>) -> IoResult<()> {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    bincode::serialize_into(writer, data)?;
    Ok(())
}

#[allow(dead_code)]
pub fn read_v_u64<P: AsRef<Path>>(path: P) -> IoResult<Vec<u64>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let data = bincode::deserialize_from(reader)?;
    Ok(data)
}

// ============================================================
// Vec<(f64, f64)>
// ============================================================

pub fn write_v_f64_pair<P: AsRef<Path>>(path: P, data: &Vec<(f64, f64)>) -> IoResult<()> {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    for (a, b) in data {
        writer.write_all(&a.to_le_bytes())?;
        writer.write_all(&b.to_le_bytes())?;
    }
    Ok(())
}

/// `write_v_f64_pair` で書き出した生リトルエンディアン形式（1 ペア = 16 bytes）を読む。
#[allow(dead_code)]
pub fn read_v_f64_pair<P: AsRef<Path>>(path: P) -> IoResult<Vec<(f64, f64)>> {
    let bytes = std::fs::read(path)?;
    if bytes.len() % 16 != 0 {
        return Err("invalid f64-pair .dat length".into());
    }
    let mut out = Vec::with_capacity(bytes.len() / 16);
    for chunk in bytes.chunks_exact(16) {
        let a = f64::from_le_bytes(chunk[0..8].try_into().unwrap());
        let b = f64::from_le_bytes(chunk[8..16].try_into().unwrap());
        out.push((a, b));
    }
    Ok(out)
}

// ============================================================
// Vec<(f64, f64, f64)>
// ============================================================

pub fn write_v_f64_triple<P: AsRef<Path>>(path: P, data: &Vec<(f64, f64, f64)>) -> IoResult<()> {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    for (a, b, c) in data {
        writer.write_all(&a.to_le_bytes())?;
        writer.write_all(&b.to_le_bytes())?;
        writer.write_all(&c.to_le_bytes())?;
    }
    Ok(())
}

// ============================================================
// CSV (text)
// ============================================================

/// ヘッダ 1 行 + 整形済みデータ行を CSV テキストとして書き出す。
#[allow(dead_code)]
pub fn write_csv<P: AsRef<Path>>(path: P, header: &str, rows: &[String]) -> IoResult<()> {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "{}", header)?;
    for row in rows {
        writeln!(writer, "{}", row)?;
    }
    Ok(())
}

// ============================================================
// 使用例
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vv_usize() {
        let original: Vec<Vec<usize>> = vec![vec![0, 1, 2], vec![3, 4], vec![5]];
        write_vv_usize("test_vv_usize.bin", &original).unwrap();
        let loaded = read_vv_usize("test_vv_usize.bin").unwrap();
        assert_eq!(original, loaded);
        std::fs::remove_file("test_vv_usize.bin").unwrap();
    }

    #[test]
    fn test_v_f64() {
        let original: Vec<f64> = vec![1.0, 2.5, std::f64::consts::PI];
        write_v_f64("test_v_f64.bin", &original).unwrap();
        let loaded = read_v_f64("test_v_f64.bin").unwrap();
        assert_eq!(original, loaded);
        std::fs::remove_file("test_v_f64.bin").unwrap();
    }

    #[test]
    fn test_v_u64() {
        let original: Vec<u64> = vec![0, u64::MAX, 12345678];
        write_v_u64("test_v_u64.bin", &original).unwrap();
        let loaded = read_v_u64("test_v_u64.bin").unwrap();
        assert_eq!(original, loaded);
        std::fs::remove_file("test_v_u64.bin").unwrap();
    }

    #[test]
    fn test_write_v_f64_pair() {
        let data: Vec<(f64, f64)> = vec![(1.0, 2.0), (3.5, 4.5), (std::f64::consts::PI, 0.0)];
        write_v_f64_pair("test_v_f64_pair.dat", &data).unwrap();
        std::fs::remove_file("test_v_f64_pair.dat").unwrap();
    }

    #[test]
    fn test_v_f64_pair_round_trip() {
        let original: Vec<(f64, f64)> = vec![(1.0, 2.0), (3.5, 4.5), (std::f64::consts::PI, -0.25)];
        write_v_f64_pair("test_v_f64_pair_rt.dat", &original).unwrap();
        let loaded = read_v_f64_pair("test_v_f64_pair_rt.dat").unwrap();
        assert_eq!(original, loaded);
        std::fs::remove_file("test_v_f64_pair_rt.dat").unwrap();
    }
}
