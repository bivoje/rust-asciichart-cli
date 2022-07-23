
use asciichart_cli::{plot, Args, Parser};
//use clap::derive::Parser;

fn main() {
    let args = Args::parse();

    let width = 80;
    let mut v = vec![0f64; width];
    for i in 0 .. width {
        let pi = std::f64::consts::PI;
        //v[i] = 3000. +  0.000001 * (i as f64 * pi * 4.0 / 120.0).sin();
        v[i] = 100. + 0.01 * (i as f64 * pi * 4.0 / width as f64).sin();
    }
    let vss = vec![(v,2)];

    let cfg = args.gen_config(&vss).unwrap(); // FIXME
    let ret = plot(&vss, cfg);
    print!("{}", ret);
}
