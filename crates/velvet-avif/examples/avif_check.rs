//! Decodes an AVIF to PNG, or frames of an AV1 IVF clip (to compare with other decoders).
//! `avif_check <file.avif> <out.png>` · `avif_check <clip.ivf> <out_dir> [frame…]`
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let input = std::path::Path::new(&args[1]);
    if input.extension().is_some_and(|e| e == "ivf") {
        let mut clip = velvet_avif::Clip::open(input, std::env::var("THREADS").ok().and_then(|v| v.parse().ok()).unwrap_or(1)).expect("clip");
        let t = std::time::Instant::now();
        let list: Vec<usize> = if args.len() > 3 { args[3..].iter().map(|a| a.parse().unwrap()).collect() } else { (0..clip.len()).collect() };
        let mut n = 0;
        for i in &list {
            let f = clip.frame(*i).expect("frame");
            if args.len() > 3 {
                f.save(format!("{}/f{:04}.png", args[2], i)).unwrap();
            }
            n += 1;
        }
        eprintln!("{} frames of {} in {:.1} ms each", n, clip.len(), t.elapsed().as_secs_f64() * 1000.0 / n as f64);
    } else {
        let bytes = std::fs::read(input).unwrap();
        let t = std::time::Instant::now();
        let img = velvet_avif::decode_avif(&bytes).expect("decode");
        eprintln!("{}x{} in {:.1} ms", img.width(), img.height(), t.elapsed().as_secs_f64() * 1000.0);
        img.save(&args[2]).unwrap();
    }
}
