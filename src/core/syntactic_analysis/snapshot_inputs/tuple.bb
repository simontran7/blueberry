func f(pair: (I32, Bool), single: (I32,), nested: ((I32, I32), ()), grouped: (I32)) -> (I32, Bool) {
    let empty = ();
    let grouped = (1);
    let one = (1,);
    let two = (1, true);
    let trailing = (1, true,);
    let first = pair.0;
    let deep = nested.0.1;
    let negated = -pair.0;
    let called = f(pair, single, nested, grouped).1;
    let mut point = (1, 2);
    point.0 = 3;
    nested.0.1 = 4;
    let missing = pair.;
    let unclosed = (1, 2;
    two
}
