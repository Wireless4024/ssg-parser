# SSG Parser

This library allow to extract SSG state as json

## Use case

> Let's say this is js content from SSG

```js
windows.__STATE__ = (function(a,b){return{hello:{bgga:b,dsa:a},other:"..."}}(1,"qnlcpb"))
```

> you can use it to extract `hello` field by using this code

```rust
use ssg_parser::ssg_ifn::{make_finder_object, parse_object};

fn main() {
	let finder = make_finder_object(b"hello:");
	// input will have to start with function
	let data = b"function(a,b){return {hello:{bgga:b,dsa:a},other:\"...\"}}(1,\"qnlcpb\")";
	let json = parse_object(data, &finder, 8);
	assert_eq!(json, Some(b"{\"bgga\":\"qnlcpb\",\"dsa\":1}".to_vec()));
}
```

## Speed
Benchmark on `i5-1135G7 @ 4.2Ghz`

| input len | json len |      time |
|----------:|---------:|----------:|
|      1474 |      110 |  0.346 µs |
|     12221 |      989 |  1.784 µs |
|    210349 |     3714 |  5.885 µs |
|   2215930 |    24383 | 55.114 µs |
| 106196054 |   344474 |  2.567 ms |