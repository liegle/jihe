# Jihe

A practise project of rendering points and curves from text, written for learning graphics and complier basic knowledge

Possible grammar:
```
t: Val { min: 0, max: 10 }
f: Fn { expr: 5 * x }
t': Var { expr: f(f(t)) }
p: Point { x: 2, y: 4 }
c: Curve { expr: (x - f(t)) ^ 2 + y ^ 2 = 9 }
l: Curve { expr: y = t' }
```


