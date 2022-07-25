
use asciichart_cli::{plot, Args, Parser};
//use clap::derive::Parser;

fn main() {
    let args = Args::parse();

    if let Some(ref demos) = args.demo {
        let vss = demo_data(demos);
        print!("{}", plot(&vss, args.gen_config(&vss).unwrap()));
        return;
    }

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

fn demo_data(demo :&str) -> Vec<(Vec<f64>,u32)> {
    match demo {
        "sincos" => demo_sincos(),
        "rand"   => demo_rand(),
        "rand4"  => demo_rand4(),
        _        => vec![],
    }
}

fn demo_sincos() -> Vec<(Vec<f64>,u32)> {
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

    vec![(v1,1), (v2,2)]
}

fn demo_rand() -> Vec<(Vec<f64>,u32)> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let width = 120;
    let mut v = vec![0f64; width];
    for i in 1 .. width {
        v[i] = v[i - 1] + (4.*rng.gen::<f64>()-2.).round();
    }

    vec![(v,9)]
}

fn demo_rand4() -> Vec<(Vec<f64>,u32)> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let width = 120;

    (0..4).map(|i| {
        let mut v = vec![0f64; width];
        v[0] = (10.*rng.gen::<f64>()-5.).round();
        for i in 1 .. width {
            v[i] = v[i - 1] + (4.*rng.gen::<f64>()-2.).round();
        }
        (v, i+1)
    }).collect::<Vec<_>>()
}
