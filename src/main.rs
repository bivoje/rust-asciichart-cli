
use asciichart_cli::{plot, Args, Parser};
//use clap::derive::Parser;

fn main() {
    let args = Args::parse();

    let mut vss = vec![];
    let mut datacnt = 0;

    //let mut last_height = 0;

    loop {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        if line.trim().is_empty() { break; }
        datacnt += 1;

        for (i,field) in line.split_whitespace().enumerate() {
            if vss.len() <= i {
                vss.push((vec![f64::NAN; datacnt-1],1+i as u32));
            }

            if let Ok(v) = field.parse::<f64>() {
                vss[i].0.push(v);
            } else {
                vss[i].0.push(f64::NAN);
            }
        }

        if args.scan {
            //print!("\x1b[{}A\x1b[0J", last_height);
            if let Some(cfg) = args.gen_config(&vss) {
                print!("\x1b[2J{}", plot(&vss, cfg));
            }
            //last_height = cfg.height
        }
    }

    if let Some(cfg) = args.gen_config(&vss) {
        //print!("\x1b[{}A\x1b[0J", last_height);
        if args.scan { print!("\x1b[2J"); }
        print!("{}", plot(&vss, cfg));
    }
}

fn demo() {
    let width = 80;
    let mut v1 = vec![0f64; width];
    for i in 0 .. width {
        let pi = std::f64::consts::PI;
        v1[i] = 3. * (i as f64 * pi * 4.0 / width as f64).sin();
    }

    let mut v2 = vec![0f64; width];
    for i in 0 .. width {
        let pi = std::f64::consts::PI;
        v2[i] = 7. * (i as f64 * pi * 4.0 / width as f64).cos();
    }

    let vss = vec![(v1,1), (v2,2)];

    let args = Args::default();
    let cfg = args.gen_config(&vss).unwrap();
    let ret = plot(&vss, cfg);
    print!("{}", ret);
}
