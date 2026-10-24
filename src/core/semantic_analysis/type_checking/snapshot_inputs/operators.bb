func f(a: I32, b: U64, c: Bool) -> Bool {
    let negated = -a;
    let literal = -5;
    let sum = a + 1 * 2 - a div 3 mod 2;
    let unsigned = 1 + b;
    let compared = a < 3 and b >= 2;
    let equal = c == not c or false;
    compared or equal
}
