func calling() {
    let t = (1, foo);
    t(5);
}

func field() {
    let t = (1, foo);
    let x = t.5;
}

func adding() {
    let t = (1, foo);
    let x = t + 1;
}
