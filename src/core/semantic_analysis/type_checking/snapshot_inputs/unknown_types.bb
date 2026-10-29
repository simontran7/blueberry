func after_loop() {
    let t = loop {};
    let y = t.0;
}

func after_diverging_if() {
    let t = if true { return; } else { return; };
    let y = t.0;
}

func after_diverging_block() {
    let t = { return; };
    let y = t.0;
}

func diverging_receiver() {
    let y = (loop {}).0;
}
