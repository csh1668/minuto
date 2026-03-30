# Minuto 언어 명세 v0.1

> **Minuto** — 에스페란토어로 "사소하다"라는 뜻

## 1. 설계 철학

- **가벼운 VM, 무거운 컴파일러**: VM은 1N개 내외의 바이트코드로 구성하며, 복잡한 타입 검사·최적화는 모두 컴파일러가 담당한다.
- **이식성**: Minuto 소스 파일을 `std` 외 의존성이 없는 Rust 단일 소스(VM + 바이트코드 임베딩)로 변환할 수 있어야 한다.
- **정적 타입**: 모든 타입은 컴파일 타임에 결정된다 (OCaml 스타일).
- **절차적 패러다임**: 클래스, 트레이트, 패턴 매칭 등은 도입하지 않는다.
- **파일 확장자**: `.min`

---

## 2. 타입 시스템

### 2.1. 원시 타입

| 타입 | 크기 | 설명 |
|------|------|------|
| `int` | 8바이트 | 부호 있는 64비트 정수 |
| `char` | 4바이트 | 유니코드 스칼라 값 (U+0000 ~ U+10FFFF, 서로게이트 제외) |
| `void` | 0바이트 | 반환값이 없음을 나타내는 타입 |

- `bool` 타입은 존재하지 않는다. 조건식에서는 `int` 또는 `char` 값이 0이면 거짓, 그 외에는 참으로 취급한다.
- 스택 위에서는 모든 원시 타입이 **8바이트 워드**로 통일된다. `char`는 스택에 올릴 때 8바이트로 확장하고, 메모리에 쓸 때 4바이트로 truncate한다.

### 2.2. 포인터 타입

| 타입 | 크기 | 설명 |
|------|------|------|
| `ptr<T>` | 8바이트 | `T` 값을 가리키는 단순 포인터 |

- 포인터 산술은 **타입 기반**이다. `ptr<int> + 1`은 주소가 `sizeof(int)` = 8바이트만큼 증가한다.
- `ptr<T>`에 대한 `[]` 인덱싱은 포인터 산술 + 역참조의 sugar이다: `p[i]` ≡ `*(p + i)`. 경계 검사를 수행하지 않는다.

### 2.3. span 타입

| 타입 | 크기 | 설명 |
|------|------|------|
| `span<T>` | 16바이트 | `T` 포인터(8바이트) + 길이(8바이트) |

- `span<T>`은 **배열 및 배열 슬라이스** 전용 타입이다.
- `.ptr`으로 내부 `ptr<T>`을, `.len`으로 길이를 접근할 수 있다.
- `span<T>`에 대한 `[]` 인덱싱은 **항상 런타임 경계 검사**를 수행한다. 범위를 벗어나면 trap이 발생한다.
- 경계 검사가 불필요한 저수준 접근이 필요하면 `.ptr`을 추출하여 `ptr<T>`로 사용한다.

```
// 방법 1: alloc의 결과를 span<T>으로 받으면 컴파일러가 자동 변환
var arr: span<int> = alloc<int>(10);  // → span::new(alloc<int>(10), 10)

// 방법 2: 명시적 span 생성
var p: ptr<int> = alloc<int>(10);
var arr2: span<int> = span::new(p, 10);

arr[0] = 42;          // 경계 검사 O
arr.ptr[0] = 42;      // 경계 검사 X (ptr<T> 접근)

var raw: ptr<int> = arr.ptr;
raw[0] = 42;          // 경계 검사 X
```

### 2.4. 합성 타입 (struct)

```
struct Point {
    x: int,
    y: int,

    fn distance_sq(self, other: ptr<Point>) -> int {
        var dx: int = other->x - self->x;
        var dy: int = other->y - self->y;
        return dx * dx + dy * dy;
    }
}
```

- C 언어의 구조체와 유사하되, **메서드를 struct 블록 내부에 직접 정의**할 수 있다.
- 필드는 선언 순서대로 메모리에 배치된다 (패딩 규칙은 추후 정의).
- 중첩 struct를 허용한다.
- 제네릭 struct는 v0.2에서 도입 예정이다.

#### 2.4.1. struct 리터럴

struct 인스턴스를 생성할 때는 이름 있는 필드를 사용한다:

```
var p: Point = Point {
    x: 10,
    y: 20,
};
```

#### 2.4.2. 메서드

메서드는 struct 블록 내부에 `fn`으로 선언하며, 첫 번째 매개변수로 `self`를 받는다.

```
struct Counter {
    count: int,

    fn new() -> Counter {
        return Counter { count: 0 };
    }

    fn increment(self) {
        self->count = self->count + 1;
    }

    fn get(self) -> int {
        return self->count;
    }
}
```

**self 규칙:**

- `self`의 타입은 생략할 수 있다. 컴파일러가 해당 struct의 `ptr<StructName>`으로 추론한다.
- 즉, `fn increment(self)` ≡ `fn increment(self: ptr<Counter>)` 이다.
- `self`가 없는 메서드는 정적 메서드이다 (예: `new()`). `StructName::method()` 형태로 호출한다.

**메서드 호출과 자동 `&` 전달:**

```
var c: Counter = Counter::new();  // 정적 메서드
c.increment();                    // → Counter::increment(&c)
var n: int = c.get();             // → Counter::get(&c)
```

- `value.method(args)` 형태로 호출하면, 컴파일러가 `self` 타입이 `ptr<T>`인 경우 자동으로 `&value`를 전달한다.
- 이는 순수 syntax sugar이며, `Counter::increment(&c)`와 동일한 바이트코드를 생성한다.

#### 2.4.3. 타입 키워드의 정적 메서드

빌트인 타입 키워드(`span`, `ptr`, `int`, `char`)도 정적 메서드를 가질 수 있다. Parser에서는 타입 키워드 뒤에 `::`이 오면 정적 메서드 호출로 해석한다.

```
var arr: span<int> = span::new(p, 10);  // 빌트인 타입의 정적 메서드
var n: int = int::parse(line);          // 빌트인 타입의 정적 메서드
var c: Counter = Counter::new();        // 사용자 struct의 정적 메서드
```

이 형태들은 동일한 문법 규칙(`TypeOrIdent :: Ident ( args )`)으로 파싱된다.

### 2.5. 함수 타입

```
fn(int, int) -> int
```

- 함수는 일급 값이다. 변수에 대입하거나 인자로 전달할 수 있다.
- 런타임에 함수 값은 코드 영역의 주소를 담은 8바이트 정수이다.
- 클로저는 지원하지 않는다 (캡처 없음).

### 2.6. readonly 수식어

```
var s: readonly span<char> = "hello";
```

- `readonly`는 `ptr<T>` 또는 `span<T>` 앞에 붙여, 해당 변수를 통한 대상 메모리의 **쓰기를 금지**하는 컴파일러 지시어이다.
- 바이트코드에는 반영되지 않으며, 컴파일 타임에만 검사한다.
- 문자열 리터럴은 상수 데이터 영역에 위치하므로 `readonly span<char>` 타입을 가진다.

#### 2.6.1. readonly widening (서브타이핑)

`span<T>` → `readonly span<T>`, `ptr<T>` → `readonly ptr<T>` 방향의 암묵적 확대(widening)를 허용한다. 이는 안전한 방향(쓰기 권한을 버리는 방향)이므로 형변환이 아닌 **서브타이핑 규칙**으로 취급한다.

```
fn print_line(s: readonly span<char>) {
    std::print("{}\n", s);
}

var owned: span<char> = std::input();   // 힙 할당, 쓰기 가능
print_line(owned);                       // span<char> → readonly span<char> 암묵적 확대

var literal: readonly span<char> = "hello";
print_line(literal);                     // 직접 전달
```

- 역방향(`readonly span<T>` → `span<T>`)은 허용하지 않는다.
- 이것은 §2.9의 "암시적 형변환은 존재하지 않는다" 규칙의 예외가 아니라, 별도의 서브타이핑 관계이다.

### 2.7. 제네릭

- 제네릭은 **monomorphization** 방식으로 처리한다.
- 컴파일러가 사용된 타입 조합마다 별도의 구체 코드를 생성한다.
- VM은 제네릭의 존재를 알지 못한다.
- v0.1에서 제네릭은 **빌트인 함수(`alloc`, `free`)와 `ptr<T>`, `span<T>` 타입에만** 사용된다.
- 사용자 정의 제네릭 함수 및 제네릭 struct는 v0.2에서 도입 예정이다.

```
// alloc<int>(5)는 컴파일러에 의해
// PUSH (8 * 5), ALLOC으로 변환된다.
```

### 2.8. 타입 추론

- **로컬 변수**에 한해 타입 추론을 지원한다.
- 함수 시그니처(매개변수 타입, 반환 타입)는 반드시 명시해야 한다. 단, 메서드의 `self` 매개변수는 타입 생략을 허용한다.
- 반환 타입이 `void`인 경우에 한해 생략을 허용한다.

```
fn add(a: int, b: int) -> int {
    var result = a + b;  // int로 추론됨
    return result;
}

fn greet() {  // -> void 생략됨
    std::print("hello\n");
}
```

### 2.9. 형변환

- 암시적 형변환은 존재하지 않는다.
- 명시적 형변환 연산을 통해 변환한다 (상세 문법은 추후 확정).

---

## 3. 변수 선언

### 3.1. var (가변)

```
var x: int = 5;
var y = 10;        // 타입 추론: int
x = 42;            // 재대입 가능
```

### 3.2. const (불변)

```
const MAX: int = 1024;
// MAX = 2048;  // 컴파일 에러
```

- `const`로 선언된 변수는 초기화 이후 재대입할 수 없다.
- 컴파일 타임 상수이며 상수 데이터 영역에 배치될 수 있다.

### 3.3. 스코프

- 블록 스코프를 따른다. `{}` 블록 내에서 선언된 변수는 블록이 끝나면 소멸한다.
- **섀도잉을 허용**한다. 내부 스코프에서 동일 이름의 변수를 재선언할 수 있다.

```
var x: int = 5;
{
    var x: int = 10;  // 섀도잉: 새 x
    std::print("{}\n", x);  // 10
}
std::print("{}\n", x);  // 5
```

---

## 4. 연산자

### 4.1. 산술 연산자

| 연산자 | 설명 | 피연산자 |
|--------|------|----------|
| `+` | 덧셈 | int, 포인터 산술 |
| `-` | 뺄셈 | int, 포인터 산술 |
| `*` | 곱셈 / 역참조 | int (이항) / ptr (단항) |
| `/` | 나눗셈 | int |
| `%` | 나머지 | int |

### 4.2. 비트 연산자

| 연산자 | 설명 |
|--------|------|
| `&` | 비트 AND / 주소 획득 (단항) |
| `\|` | 비트 OR |
| `^` | 비트 XOR |
| `<<` | 왼쪽 시프트 |
| `>>` | 오른쪽 시프트 (산술) |
| `~` | 비트 NOT (단항) |

### 4.3. 비교 연산자

| 연산자 | 설명 |
|--------|------|
| `==` | 같음 |
| `!=` | 다름 |
| `<` | 작음 |
| `<=` | 작거나 같음 |
| `>` | 큼 |
| `>=` | 크거나 같음 |

- 비교 연산의 결과는 `int`이다. 참이면 `1`, 거짓이면 `0`.

### 4.4. 논리 연산자

| 연산자 | 설명 |
|--------|------|
| `&&` | 논리 AND (단축 평가) |
| `\|\|` | 논리 OR (단축 평가) |
| `!` | 논리 NOT (단항) |

- 피연산자는 `int`이며, 0이면 거짓, 그 외에는 참으로 취급한다.
- `&&`와 `||`는 단축 평가(short-circuit evaluation)를 수행한다.

### 4.5. 포인터 연산자

| 연산자 | 설명 | 예시 |
|--------|------|------|
| `&` (단항) | 변수의 주소를 얻음 | `&x` → `ptr<int>` |
| `*` (단항) | 포인터 역참조 | `*p` → `int` |
| `->` | 포인터를 통한 필드 접근 | `p->x` ≡ `(*p).x` |
| `[]` | 인덱싱 | `p[i]` — ptr: 검사 없음, span: 경계 검사 |
| `.` | 필드/메서드 접근 | `s.len`, `c.get()` |

### 4.6. 대입

```
x = 10;
```

- 복합 대입 연산자 (`+=`, `-=` 등)는 현재 미지원. 추후 논의.
- 증감 연산자 (`++`, `--`)는 지원하지 않는다.

### 4.7. 연산자 우선순위 (높은 것부터)

| 순위 | 연산자 | 결합 방향 |
|------|--------|-----------|
| 1 | `()` `[]` `->` `.` | 좌 → 우 |
| 2 | `!` `~` `-` (단항) `*` (역참조) `&` (주소) | 우 → 좌 |
| 3 | `*` `/` `%` | 좌 → 우 |
| 4 | `+` `-` | 좌 → 우 |
| 5 | `<<` `>>` | 좌 → 우 |
| 6 | `<` `<=` `>` `>=` | 좌 → 우 |
| 7 | `==` `!=` | 좌 → 우 |
| 8 | `&` (비트) | 좌 → 우 |
| 9 | `^` | 좌 → 우 |
| 10 | `\|` | 좌 → 우 |
| 11 | `&&` | 좌 → 우 |
| 12 | `\|\|` | 좌 → 우 |
| 13 | `=` | 우 → 좌 |

---

## 5. 제어 흐름

### 5.1. if / else

```
if condition {
    // ...
} else if condition2 {
    // ...
} else {
    // ...
}
```

- 조건식의 괄호는 선택사항이다 (Rust 스타일).
- 조건식은 `int` 또는 `char` 값으로 평가되며, 0이면 거짓, 그 외에는 참이다.
- 중괄호 `{}`는 필수이다.

### 5.2. while

```
while condition {
    // ...
}
```

- `for` 루프는 현재 미지원. 추후 range 타입과 함께 도입 예정.

### 5.3. break / continue

```
while 1 {
    if done {
        break;
    }
    if skip {
        continue;
    }
    // ...
}
```

### 5.4. return

```
fn foo() -> int {
    return 42;
}

fn bar() {
    // void 함수에서는 값 없이 return
    return;
}
```

- 함수의 마지막 문장에 도달하면 `void` 함수는 암묵적으로 반환한다.
- `void`가 아닌 함수에서 `return`을 빠뜨리면 컴파일 에러이다.

---

## 6. 함수

### 6.1. 선언

```
fn name(param1: Type1, param2: Type2) -> ReturnType {
    // ...
}
```

- 반환 타입이 `void`이면 `-> void`를 생략할 수 있다.
- 매개변수 타입은 반드시 명시해야 한다 (메서드의 `self`는 예외, §2.4.2 참조).

### 6.2. 일급 함수

```
fn add(a: int, b: int) -> int {
    return a + b;
}

fn apply(f: fn(int, int) -> int, x: int, y: int) -> int {
    return f(x, y);
}

fn main() {
    var op: fn(int, int) -> int = add;
    std::print("{}\n", apply(op, 3, 4));  // 7
}
```

- 함수 값은 코드 영역의 주소(8바이트)이다.
- 간접 호출은 `CALL_IND` 바이트코드로 구현된다.
- 클로저는 지원하지 않는다.
- 메서드도 일급 함수로 참조할 수 있다: `Counter::increment`는 `fn(ptr<Counter>)` 타입의 함수 값이다.

### 6.3. 진입점

- 프로그램의 진입점은 `main` 함수이다.
- `main`의 시그니처는 `fn main()` (반환 타입 void)이다.

---

## 7. 메모리 모델

### 7.1. VM 메모리 영역

| 영역 | 설명 |
|------|------|
| **런타임 레이어** | VM 자체 상태 관리 |
| **코드 영역** | 바이트코드가 저장되는 읽기 전용 영역 |
| **상수 데이터 영역** | 문자열 리터럴, const 값 등 읽기 전용 데이터 |
| **스택** | 지역 변수, 함수 인자, 임시 값. 프레임 제한 256개 |
| **힙** | 동적 메모리. alloc/free로 관리 |

- 모든 메모리는 VM 내부에서 통합 관리하며, Rust의 메모리 관리와 독립적이다.
- 힙 할당자는 VM에 내장한다.

### 7.2. 힙 할당 빌트인

| 함수 | 설명 |
|------|------|
| `alloc<T>(n: int) -> ptr<T>` | `T` 타입 `n`개 크기의 힙 메모리를 할당하고 포인터를 반환 |
| `free(p: ptr<T>)` | ptr이 가리키는 메모리를 해제 |
| `span::new(p: ptr<T>, len: int) -> span<T>` | ptr과 길이를 묶어 span 값을 생성 (힙 할당 아님) |

- `alloc`과 `free`는 **키워드가 아닌 빌트인 식별자**이다. 렉서에서 일반 식별자(`Ident`)로 토큰화되며, Parser가 이름으로 구분하여 특수 파싱 경로로 분기한다.
- Resolver에서 `alloc`, `free` 이름의 변수/함수 재정의는 금지한다 (`ReservedIdentifier` 에러).
- `span::new`는 빌트인 타입 `span`의 정적 메서드이다 (§2.4.3 참조). `span` 타입 키워드 뒤에 `::`이 오면 정적 메서드 호출로 파싱한다.
- 컴파일러는 `T`의 크기를 컴파일 타임에 계산하여 바이트코드에 반영한다.
- partial free는 지원하지 않는다.

#### 7.2.1. span 자동 변환 (컴파일러 sugar)

좌변 타입이 `span<T>`인 대입에서 우변이 `alloc<T>(n)` 호출이면, 컴파일러가 자동으로 `span::new(alloc<T>(n), n)`으로 변환한다:

```
// 이 둘은 동일한 바이트코드를 생성한다:
var arr: span<int> = alloc<int>(n);                // sugar
var arr: span<int> = span::new(alloc<int>(n), n);  // 명시적
```

마찬가지로, `free`에 `span<T>` 값을 전달하면 컴파일러가 자동으로 `.ptr`을 추출한다:

```
// 이 둘은 동일한 바이트코드를 생성한다:
free(arr);       // sugar — arr이 span<T>이면 자동으로 arr.ptr 추출
free(arr.ptr);   // 명시적
```

이 sugar는 역방향 타입 추론의 유일한 적용 사례이며, `alloc`/`free`에 대해서만 동작한다.

### 7.3. 포인터 안전성

| 상황 | 동작 |
|------|------|
| dangling pointer 접근 | **undefined** (C 언어와 동일) |
| double free | **런타임 에러 (trap)** |
| span 경계 초과 접근 (`[]`) | **런타임 에러 (trap)** |
| null pointer 역참조 | **런타임 에러 (trap)** |
| 스택 변수의 주소 획득 | **허용** (`&` 연산자 사용) |
| 스택 주소의 escape | **undefined** (함수 반환 후 dangling) |

---

## 8. 문자열

- 문자열 리터럴의 타입은 `readonly span<char>`이다.
- 상수 데이터 영역에 저장되며, `span`의 길이 필드에 문자 수가 포함된다.
- `readonly` 수식어에 의해 문자열 리터럴을 통한 쓰기는 컴파일 에러이다.
- 표준 라이브러리에서 `string` struct를 제공할 예정이다 (v0.1에서는 미포함).

```
var s: readonly span<char> = "hello";
std::print("{}\n", s);     // hello
std::print("{}\n", s.len); // 5
// s[0] = 'H';             // 컴파일 에러: readonly
```

---

## 9. 빌트인 / std

v0.1에서 `std`는 빌트인으로 제공하며, 별도 모듈 시스템은 없다.

### 9.1. 입출력

| 함수 | 설명 |
|------|------|
| `std::print(fmt, ...)` | 포맷 문자열을 stdout에 출력 |
| `std::input() -> span<char>` | stdin에서 한 줄을 읽어 반환 |

- 포맷 문자열에서 `{}`는 순서대로 인자를 삽입한다.
- 포맷 문자열 파싱은 **컴파일 타임**에 수행하여, VM에는 개별 SYSCALL 시퀀스로 변환한다.

#### 9.1.1. std::input

`std::input()`은 stdin에서 **한 줄**의 텍스트를 읽어 `span<char>`로 반환한다.

- 반환된 `span<char>`는 **힙에 할당**된 메모리를 가리킨다. 호출자가 소유권을 가지며, 사용 후 `free()`로 해제해야 한다.
- `readonly`가 아니므로 쓰기가 가능하다 (문자열 리터럴과 달리 힙 메모리이므로).
- 후행 개행 문자(`\n`)는 포함되지 않는다.
- 여러 줄이 한번에 입력되더라도(붙여넣기 등) 한 번의 `std::input()` 호출은 한 줄만 반환한다. 나머지 줄은 호스트 있는 입력 버퍼에 남아 다음 `std::input()` 호출 시 제공된다.

```
var line: span<char> = std::input();    // 힙 할당, 호출자 소유
var n: int = int::parse(line);          // 파싱
free(line);                             // 명시적 해제
```

**메모리 소유권 요약:**

| 표현 | 타입 | 소유권 | free |
|------|------|--------|------|
| `"hello"` | `readonly span<char>` | 상수 데이터 영역 | 불필요/불가 |
| `std::input()` | `span<char>` | 힙 (호출자) | 필요 |
| `alloc<char>(n)` → span | `span<char>` | 힙 (호출자) | 필요 |

#### 9.1.2. std::print 포맷 검증

TypeChecker에서 `std::print` 호출에 대해 다음을 검사한다:

1. 첫 번째 인자가 **문자열 리터럴**이어야 한다. 변수나 표현식은 불가.
2. 포맷 문자열 내 `{}` 플레이스홀더 개수와 나머지 인자 개수가 일치해야 한다.
3. 각 인자의 타입이 printable해야 한다. printable 타입: `int`, `char`, `span<char>` (및 `readonly span<char>`).

```
std::print("{}\n", 42);              // OK: int
std::print("{}\n", 'a');             // OK: char
std::print("{}\n", "hello");         // OK: readonly span<char>
// std::print("{} {}\n", 42);        // 에러 E0316: 플레이스홀더 2개, 인자 1개
// var fmt = "{}"; std::print(fmt);  // 에러 E0315: 첫 인자가 리터럴이 아님
```

CodeGen에서는 포맷 문자열을 분해하여, 리터럴 구간은 `SYSCALL(PrintStr)`로, `{}` 구간은 인자 타입에 따라 `SYSCALL(PrintInt)` / `SYSCALL(PrintChar)` / `SYSCALL(PrintStr)`로 emit한다.

#### 9.1.3. 파싱 빌트인

텍스트 입력을 값으로 변환하는 빌트인 함수를 제공한다.

| 함수 | 설명 |
|------|------|
| `int::parse(s: readonly span<char>) -> int` | 문자열을 정수로 파싱 |
| `char::parse(s: readonly span<char>) -> char` | 문자열을 문자로 파싱 |

- `int::parse`는 선행/후행 공백을 무시하고, 선택적 `'-'` 부호 및 숫자 문자열을 64비트 정수로 변환한다.
- `char::parse`는 정확히 1개의 문자로 구성된 문자열을 `char` 값으로 변환한다.
- 파싱 실패 시 `ParseError` trap이 발생한다 (§10.1 참조).
- 매개변수 타입이 `readonly span<char>`이므로, `span<char>`와 `readonly span<char>` 모두 전달할 수 있다 (§2.6.1 readonly widening 참조).
- 이 함수들은 컴파일러가 바이트코드로 생성하는 빌트인 루틴이다. 해당 함수가 사용될 때만 코드 영역에 emit된다.

```
// 사용 예시
var line: span<char> = std::input();
var n: int = int::parse(line);          // 성공: "42" → 42
free(line);

var lit: readonly span<char> = "97";
var c: char = char::parse(lit);         // readonly span<char>도 전달 가능
```

### 9.2. VM I/O 모델 (Effect 기반)

VM의 I/O는 **Effect 기반** 모델로 동작한다. VM은 실제 I/O를 직접 수행하지 않고, SYSCALL을 만나면 호스트 프로그램에 **제어권을 양보(yield)**한다.

```
VM 실행 흐름:
  vm.run_until_effect()
    │
    ├─ 일반 opcode → VM 내부에서 처리, 계속 실행
    ├─ SYSCALL(PrintXxx) → VmEffect::Output 반환 (단방향, 응답 불필요)
    ├─ SYSCALL(ReadLine) → VmEffect::Input 반환 (양방향, provide_input 필요)
    ├─ HALT → VmEffect::Halted 반환
    └─ 런타임 에러 → VmEffect::Error 반환
```

**VmEffect 종류:**

| Effect | 방향 | 설명 |
|--------|------|------|
| `Output(OutputData)` | VM → 호스트 | 데이터 출력 요청. 호스트는 처리 후 `run_until_effect()`를 다시 호출. |
| `Input` | VM ↔ 호스트 | 한 줄 입력 요청. 호스트는 `provide_input(line)`으로 응답 후 `run_until_effect()` 호출. |
| `Halted` | VM → 호스트 | 프로그램 정상 종료. |
| `Error(VmError)` | VM → 호스트 | 런타임 에러 발생. |

**설계 원칙:**

- **VM 무의존성**: VM은 stdout/stdin을 직접 접근하지 않는다. I/O의 실제 처리는 전적으로 호스트 책임이다.
- **호스트 무상태**: 호스트는 타입 파싱 로직을 갖지 않는다. `std::input()`은 항상 텍스트 한 줄을 요청하고, 파싱은 컴파일러가 생성한 바이트코드가 담당한다.
- **WASM 호환성**: 블로킹 I/O가 불가능한 환경에서도 `run_until_effect()` → 콜백 → `provide_input()` 패턴으로 자연스럽게 동작한다.

**provide_input 메커니즘:**

`VmEffect::Input`에 대해 호스트가 `provide_input(line)`을 호출하면, VM은 내부적으로:
1. 힙 할당자로 `char` 배열 메모리를 할당한다 (`alloc`과 동일한 경로).
2. 입력 문자열을 VM 힙 메모리에 기록한다.
3. 스택에 `span(ptr, len)`을 push한다.

이 힙 할당은 `alloc` 빌트인과 동일한 할당자를 사용하므로, `free()`로 해제하는 것이 완전히 일관적이다.

### 9.3. 모듈 시스템

- v0.1에서는 **단일 파일만 지원**한다.
- 사용자 정의 모듈, import/use 구문은 추후 도입 예정이다.

---

## 10. 에러 처리

### 10.1. 런타임 에러 (trap)

아래 상황에서 VM은 에러 메시지를 출력하고 즉시 종료한다.

| 에러 | 설명 |
|------|------|
| `DivisionByZero` | 0으로 나누기 |
| `NullPointerAccess` | null 포인터 역참조 |
| `DoubleFree` | 이미 해제된 메모리를 다시 해제 |
| `OutOfBounds` | span 인덱싱 경계 초과 |
| `StackOverflow` | 스택 프레임 256개 초과 |
| `OutOfMemory` | 힙 메모리 부족 |
| `ParseError` | `int::parse` 또는 `char::parse` 파싱 실패 |

### 10.2. 향후 계획

- C 스타일 에러 코드를 일단 사용하되, 추후 try-catch 예외 처리 도입을 검토한다.

---

## 11. 주석

```
// 한 줄 주석

/* 여러 줄
   주석 */
```

---

## 12. 바이트코드 ISA

스택 기반 VM으로, 모든 스택 슬롯은 8바이트 워드이다.

### 12.1. Opcode 목록 (17개)

#### 스택 조작

| Opcode | 인자 | 설명 |
|--------|------|------|
| `PUSH` | imm64 | 즉시값(또는 상수 풀 인덱스)을 스택에 push |
| `POP` | - | 스택 탑 제거 |
| `DUP` | - | 스택 탑 복제 |

#### 메모리 접근

| Opcode | 인자 | 설명 |
|--------|------|------|
| `LOAD` | - | pop addr → push mem[addr] (8바이트) |
| `STORE` | - | pop val, pop addr → mem[addr] = val |
| `LOCAL` | offset | 현재 프레임 베이스 + offset 주소를 push |

#### 연산

| Opcode | 인자 | 설명 |
|--------|------|------|
| `BINOP` | op | pop a, pop b → push (b op a) |
| `UNOP` | op | pop a → push (op a) |

- `BINOP op`: add, sub, mul, div, mod, and, or, xor, shl, shr, eq, ne, lt, le, gt, ge
- `UNOP op`: neg, not, itoc, ctoi

#### 제어 흐름

| Opcode | 인자 | 설명 |
|--------|------|------|
| `JMP` | addr | 무조건 점프 |
| `JZ` | addr | pop → 0이면 점프 |
| `CALL` | addr | 리턴 주소 push + 프레임 셋업 + 점프 |
| `CALL_IND` | - | pop addr → 간접 호출 (일급 함수용) |
| `RET` | - | 프레임 정리 + 리턴 주소로 복귀 |

#### 시스템

| Opcode | 인자 | 설명 |
|--------|------|------|
| `ALLOC` | - | pop size → 힙 할당 → push ptr |
| `FREE` | - | pop ptr → 힙 해제 |
| `SYSCALL` | id | I/O 등 빌트인 호출 (§12.5 참조) |
| `HALT` | - | 프로그램 종료 |

### 12.2. span 처리

- `span<T>`은 스택에서 2슬롯(16바이트)을 차지한다.
- 첫 번째 슬롯은 `ptr<T>` (주소), 두 번째 슬롯은 길이 (`int`).
- `span::new(alloc<T>(n), n)` sugar는 ALLOC 후 길이를 추가 push하는 시퀀스로 구현한다.
- `.len` 접근은 두 번째 슬롯을 읽는 LOCAL 명령으로 변환한다.
- `.ptr` 접근은 첫 번째 슬롯을 읽는 LOCAL 명령으로 변환한다.
- `[]` 인덱싱 시 경계 검사 코드를 컴파일러가 삽입한다:

```
// arr[i] 에 대한 바이트코드 (span<int>)
// 1. 경계 검사
LOCAL <arr_len_offset>   // arr.len 주소
LOAD                     // arr.len 값
LOCAL <i_offset>         // i 주소
LOAD                     // i 값
BINOP lt                 // i < arr.len ?
JZ <trap_out_of_bounds>  // 아니면 trap

// 2. 실제 접근
LOCAL <arr_ptr_offset>   // arr.ptr 주소
LOAD                     // arr.ptr 값
LOCAL <i_offset>
LOAD
PUSH 8                   // sizeof(int)
BINOP mul
BINOP add                // arr.ptr + i * sizeof(int)
LOAD                     // 값 로드
```

### 12.3. struct 필드 접근

ptr 산술 + LOAD/STORE 조합으로 구현한다.

```
// struct Point { x: int, y: int }
// var p: ptr<Point> = alloc<Point>(1);
// p->y = 42;

LOCAL <p_offset>   // p의 로컬 변수 주소
LOAD               // p의 값 (힙 주소)
PUSH 8             // y 필드 오프셋 (x가 8바이트)
BINOP add          // 주소 + 오프셋
PUSH 42
STORE
```

### 12.4. 메서드 호출

메서드 호출은 자동 `&` 전달을 포함한 일반 함수 호출로 변환된다.

```
// c.increment() 에 대한 바이트코드
// → Counter::increment(&c)

LOCAL <c_offset>         // c의 주소를 push (&c)
CALL <Counter_increment> // 함수 호출
```

### 12.5. SYSCALL ID

`SYSCALL id` 명령의 `id`는 VM이 호스트에 돌려보낼 VmEffect의 종류를 결정한다.

#### 출력 (VM → 호스트, 응답 불필요)

| ID | 이름 | 동작 | VmEffect |
|----|------|------|----------|
| `0x01` | `PrintInt` | pop int → 정수 출력 | `Output(Int(v))` |
| `0x02` | `PrintChar` | pop char → 문자 출력 | `Output(Char(v))` |
| `0x03` | `PrintStr` | pop len, pop ptr → 문자열 출력 | `Output(Str { addr, len })` |

#### 입력 (VM ↔ 호스트, 응답 필요)

| ID | 이름 | 동작 | VmEffect |
|----|------|------|----------|
| `0x10` | `ReadLine` | 한 줄 입력 요청 → `provide_input` 후 span(ptr, len) push | `Input` |

- 입력 SYSCALL은 `ReadLine` **하나만** 존재한다. `std::input<int>()`/`std::input<char>()` 같은 타입별 입력 구분은 없다.
- 컴파일러는 `std::input()`을 `SYSCALL(ReadLine)`으로 emit하고, 파싱이 필요하면 `ReadLine` 후에 `int::parse`/`char::parse` 빌트인 루틴 호출을 추가로 emit한다.
- 호스트는 타입 파싱을 수행하지 않는다. 항상 텍스트 한 줄만 제공한다.

#### std::print CodeGen 예시

```
// std::print("{} + {} = {}\n", a, b, a + b)
// 컴파일러가 다음 SYSCALL 시퀀스로 분해:

LOCAL <a>                // a 값
LOAD
SYSCALL PrintInt         // 출력: a
PUSH <const_" + ">       // 상수 데이터 주소
PUSH 3                   // 길이
SYSCALL PrintStr         // 출력: " + "
LOCAL <b>
LOAD
SYSCALL PrintInt         // 출력: b
PUSH <const_" = ">
PUSH 3
SYSCALL PrintStr         // 출력: " = "
// ... (a + b 계산 후)
SYSCALL PrintInt         // 출력: a + b
PUSH <const_"\n">
PUSH 1
SYSCALL PrintStr         // 출력: "\n"
```

---

## 13. VM 구조

```
┌─────────────────────────────────────────┐
│                  VM                     │
├──────────┬──────────┬──────┬────────────┤
│   Code   │ Const    │Stack │    Heap    │
│  (R/O)   │ Data(R/O)│      │            │
│          │          │      │            │
│ bytecode │ string   │ 256  │  alloc /   │
│ sequence │ literals │frames│  free      │
│          │ consts   │ max  │  managed   │
└──────────┴──────────┴──────┴────────────┘
         ▲                        ▲
         │    ┌────────────┐      │
         └────│  Allocator │──────┘
              │ (built-in) │
              └────────────┘
```

- **프로그램 카운터 (PC)**: 현재 실행 중인 바이트코드 주소
- **스택 포인터 (SP)**: 스택 탑 위치
- **프레임 포인터 (FP)**: 현재 스택 프레임의 베이스 주소
- **프레임 제한**: 최대 256개

---

## 14. 예제 프로그램

### 14.1. 피보나치 (재귀)

```
fn fib(n: int) -> int {
    if n <= 1 {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

fn main() {
    var line: span<char> = std::input();
    var n: int = int::parse(line);
    free(line);
    std::print("{}\n", fib(n));
}
```

### 14.2. 피보나치 (반복)

```
fn fib(n: int) -> int {
    if n <= 1 {
        return n;
    }
    var a: int = 0;
    var b: int = 1;
    var i: int = 2;
    while i <= n {
        var tmp: int = a + b;
        a = b;
        b = tmp;
        i = i + 1;
    }
    return b;
}

fn main() {
    var line: span<char> = std::input();
    var n: int = int::parse(line);
    free(line);
    std::print("{}\n", fib(n));
}
```

### 14.3. 동적 배열 (span)

```
fn main() {
    var line: span<char> = std::input();
    var n: int = int::parse(line);
    free(line);

    var arr: span<int> = alloc<int>(n);  // span<T>으로 받으면 자동 변환

    arr[0] = 0;
    if n > 1 {
        arr[1] = 1;
    }

    var i: int = 2;
    while i < arr.len {
        arr[i] = arr[i - 1] + arr[i - 2];
        i = i + 1;
    }

    i = 0;
    while i < arr.len {
        std::print("{} ", arr[i]);
        i = i + 1;
    }
    std::print("\n");

    free(arr);  // span이어도 그냥 free (자동으로 arr.ptr 추출)
}
```

### 14.4. struct + 메서드 + 포인터

```
struct Point {
    x: int,
    y: int,

    fn new(x: int, y: int) -> ptr<Point> {
        var p: ptr<Point> = alloc<Point>(1);
        p->x = x;
        p->y = y;
        return p;
    }

    fn distance_sq(self, other: ptr<Point>) -> int {
        var dx: int = other->x - self->x;
        var dy: int = other->y - self->y;
        return dx * dx + dy * dy;
    }

    fn scale(self, factor: int) {
        self->x = self->x * factor;
        self->y = self->y * factor;
    }
}

fn main() {
    var a: ptr<Point> = Point::new(1, 2);
    var b: ptr<Point> = Point::new(4, 6);

    std::print("dist^2 = {}\n", a->distance_sq(b));

    a->scale(3);
    std::print("a = ({}, {})\n", a->x, a->y);  // a = (3, 6)

    free(a);
    free(b);
}
```

### 14.5. 주소 연산자 & 역참조

```
fn swap(a: ptr<int>, b: ptr<int>) {
    var tmp: int = *a;
    *a = *b;
    *b = tmp;
}

fn main() {
    var x: int = 10;
    var y: int = 20;
    swap(&x, &y);
    std::print("x={}, y={}\n", x, y);  // x=20, y=10
}
```

### 14.6. struct 리터럴 + 메서드 + span 종합

```
struct Counter {
    count: int,

    fn new() -> Counter {
        return Counter { count: 0 };
    }

    fn increment(self) {
        self->count = self->count + 1;
    }

    fn get(self) -> int {
        return self->count;
    }
}

fn main() {
    // 값 타입 struct + 메서드 호출
    var c: Counter = Counter::new();
    c.increment();
    c.increment();
    c.increment();
    std::print("count = {}\n", c.get());  // count = 3

    // span과 ptr의 차이
    var safe: span<int> = alloc<int>(5);
    var raw: ptr<int> = safe.ptr;

    safe[0] = 100;   // 경계 검사 O
    raw[0] = 200;    // 경계 검사 X, *(raw + 0) = 200

    std::print("{}\n", safe[0]);  // 200 (같은 메모리)

    free(safe);
}
```

### 14.7. 일급 함수

```
fn add(a: int, b: int) -> int {
    return a + b;
}

fn mul(a: int, b: int) -> int {
    return a * b;
}

fn apply(f: fn(int, int) -> int, x: int, y: int) -> int {
    return f(x, y);
}

fn main() {
    std::print("{}\n", apply(add, 3, 4));  // 7
    std::print("{}\n", apply(mul, 3, 4));  // 12

    var op: fn(int, int) -> int = add;
    std::print("{}\n", op(10, 20));  // 30
}
```

---

## 15. 컴파일러 파이프라인

### 15.1. 단계 개요

```
소스 코드 (.min)
  │
  ▼
┌──────────┐    토큰 스트림
│  Lexer   │──────────────▶
└──────────┘
  │
  ▼
┌──────────┐    AST (Unresolved)
│  Parser  │──────────────▶
└──────────┘
  │
  ▼
┌──────────┐    AST (Resolved: 모든 이름에 SymbolId 부착)
│ Resolver │──────────────▶
└──────────┘
  │
  ▼
┌────────────┐  AST (Typed: 모든 노드에 타입 정보 부착)
│ TypeChecker│──────────────▶
└────────────┘
  │
  ▼
┌────────────────┐  (선택) MIR
│ MIR Generator  │──────────────▶
└────────────────┘
  │
  ▼
┌────────────────┐  바이트코드
│ Code Generator │──────────────▶
└────────────────┘
```

### 15.2. Resolver

스코프와 이름 해결을 담당한다. AST의 모든 이름 참조를 고유한 `SymbolId`로 변환한다.

**수행 작업:**

- 스코프 스택 관리 (`{` push, `}` pop)
- 변수 선언 시 새 `SymbolId` 발급
- 변수 사용 시 스코프를 거슬러 올라가며 `SymbolId` 찾기
- 미정의 변수 에러 보고
- **예약된 빌트인 식별자(`alloc`, `free`) 재정의 금지**: 이 이름들을 변수·함수 이름으로 선언하면 `ReservedIdentifier` 에러 보고
- `self`를 해당 struct의 `ptr<StructName>` 매개변수로 해석
- `c.increment()` → `Counter::increment` 메서드 해결
- `span::new` → 빌트인 타입의 정적 메서드로 해결
- `std::input` → 빌트인 입력 함수로 해결
- `int::parse`, `char::parse` → 빌트인 타입의 정적 메서드로 해결

### 15.3. TypeChecker

타입 검사 및 타입 추론을 담당한다. Resolved AST의 모든 노드에 타입 정보를 부착한다.

**수행 작업:**

- 이항 연산자 양쪽 타입 일치 검사
- `ptr<T>`에만 포인터 산술 허용
- `span<T>`의 `[]`에 경계 검사 표시 (→ MIR / CodeGen에서 사용)
- 함수 호출 시 인자 타입/개수 검사
- `readonly` 위반 검사 (`readonly span`에 쓰기 시도)
- `var x = expr`의 타입 추론
- `alloc<T>(n)`의 좌변이 `span<T>`이면 자동 span 래핑 표시
- `a.method()` → `&a` 자동 전달 타입 확인
- **`std::print` 포맷 검증**: 첫 인자 리터럴 여부, `{}` 개수와 인자 개수 매칭, 각 인자의 printable 타입 검사 (§9.1.2 참조)
- **`span::new` 타입 검사**: `span::new(p: ptr<T>, len: int) -> span<T>` 시그니처로 인자 검사
- **`std::input` 타입 검사**: `std::input() -> span<char>` 시그니처 검사 (인자 없음)
- **`int::parse`, `char::parse` 타입 검사**: `int::parse(s: readonly span<char>) -> int`, `char::parse(s: readonly span<char>) -> char` 시그니처 검사. 인자의 readonly widening 허용 (§2.6.1)

---

## 16. 컴파일 에러 분류

### 16.1. Lexer 에러

| 에러 | 코드 | 설명 | 예시 |
|------|------|------|------|
| `UnexpectedCharacter` | E0001 | 언어에 정의되지 않은 문자 | `@`, `#` |
| `InvalidIntLiteral` | E0002 | 정수 리터럴 파싱 실패 (오버플로 등) | `999999999999999999999` |
| `InvalidCharLiteral` | E0003 | 문자 리터럴 형식 오류 | `''`, `'ab'` |
| `InvalidEscapeSequence` | E0004 | 인식할 수 없는 이스케이프 시퀀스 | `'\q'` |
| `UnterminatedString` | E0005 | 닫히지 않은 문자열 리터럴 | `"hello` |
| `UnterminatedBlockComment` | E0006 | 닫히지 않은 블록 주석 | `/* ...` |

### 16.2. Parser 에러

| 에러 | 코드 | 설명 | 예시 |
|------|------|------|------|
| `UnexpectedToken` | E0101 | 기대한 토큰과 다른 토큰이 나옴 | `fn 42()` — 식별자 기대, 정수 발견 |
| `UnexpectedEof` | E0102 | 파일이 예상보다 일찍 끝남 | `fn foo(` — `)` 기대 |
| `ExpectedExpression` | E0103 | 표현식이 와야 할 위치에 다른 것이 나옴 | `var x = ;` |
| `ExpectedType` | E0104 | 타입이 와야 할 위치에 다른 것이 나옴 | `var x: = 5;` |
| `ExpectedIdentifier` | E0105 | 식별자가 와야 할 위치에 다른 것이 나옴 | `var 42 = 5;` |
| `ExpectedSemicolon` | E0106 | 문장 끝에 `;`가 없음 | `var x = 5` |
| `ExpectedBlock` | E0107 | `{`로 시작하는 블록이 필요함 | `if x return;` |
| `InvalidAssignmentTarget` | E0108 | 대입 좌변이 유효하지 않음 | `(a + b) = 5;` |

### 16.3. Resolver 에러

| 에러 | 코드 | 설명 | 예시 |
|------|------|------|------|
| `UndefinedVariable` | E0201 | 선언되지 않은 변수 참조 | `x = 5;` (x 미선언) |
| `UndefinedFunction` | E0202 | 선언되지 않은 함수 호출 | `foo();` (foo 미선언) |
| `UndefinedType` | E0203 | 선언되지 않은 타입 사용 | `var x: Foo = ...;` |
| `UndefinedField` | E0204 | struct에 존재하지 않는 필드 접근 | `p->z` (Point에 z 필드 없음) |
| `UndefinedMethod` | E0205 | struct에 존재하지 않는 메서드 호출 | `c.reset()` (Counter에 reset 없음) |
| `DuplicateDefinition` | E0206 | 같은 스코프에서 이름 중복 선언 | `var x = 1; var x = 2;` (같은 스코프) |
| `MainNotFound` | E0207 | 진입점 `main` 함수가 없음 | (파일에 main 함수 없음) |
| `InvalidMainSignature` | E0208 | `main` 함수 시그니처가 `fn main()`이 아님 | `fn main(x: int)` |
| `ReservedIdentifier` | E0209 | 예약된 빌트인 이름을 변수/함수 이름으로 사용 | `var alloc = 5;`, `fn free() {}` |

### 16.4. TypeChecker 에러

| 에러 | 코드 | 설명 | 예시 |
|------|------|------|------|
| `TypeMismatch` | E0301 | 기대 타입과 실제 타입 불일치 | `var x: int = "hello";` |
| `InvalidBinaryOp` | E0302 | 이항 연산자에 호환되지 않는 타입 조합 | `"hello" + 5` |
| `InvalidUnaryOp` | E0303 | 단항 연산자에 적용 불가능한 타입 | `!ptr` |
| `InvalidDereference` | E0304 | `ptr<T>`가 아닌 타입을 역참조 | `*x` (x가 int) |
| `InvalidFieldAccess` | E0305 | struct/ptr\<struct\>가 아닌 타입에 필드 접근 | `x.field` (x가 int) |
| `InvalidIndexing` | E0306 | `ptr<T>` 또는 `span<T>`가 아닌 타입에 인덱싱 | `x[0]` (x가 int) |
| `InvalidFunctionCall` | E0307 | 함수 타입이 아닌 값을 호출 | `x()` (x가 int) |
| `WrongArgCount` | E0308 | 함수 호출 시 인자 개수 불일치 | `add(1)` (2개 기대) |
| `InvalidPointerArithmetic` | E0309 | `ptr<T>`가 아닌 타입에 포인터 산술 시도 | `span_val + 1` |
| `AssignToConst` | E0310 | `const` 변수에 재대입 시도 | `MAX = 2048;` |
| `AssignToReadonly` | E0311 | `readonly` 참조를 통한 쓰기 시도 | `s[0] = 'H';` (s가 readonly span) |
| `MissingReturn` | E0312 | 반환 타입이 void가 아닌 함수에서 return 누락 | `fn foo() -> int { }` |
| `BreakOutsideLoop` | E0313 | 루프 밖에서 `break` 사용 | `fn foo() { break; }` |
| `ContinueOutsideLoop` | E0314 | 루프 밖에서 `continue` 사용 | `fn foo() { continue; }` |
| `PrintFormatMustBeStringLiteral` | E0315 | `std::print` 첫 인자가 문자열 리터럴이 아님 | `var f = "{}"; std::print(f, 1);` |
| `PrintArgCountMismatch` | E0316 | 포맷 `{}` 개수와 인자 개수 불일치 | `std::print("{} {}\n", 42);` |
| `PrintArgNotPrintable` | E0317 | 인자 타입이 포맷 출력 불가능 | `std::print("{}\n", some_ptr);` |

---

## 17. 향후 과제 (v0.1 범위 밖)

- [ ] 제네릭 struct / 제네릭 함수 (v0.2)
- [ ] std 라이브러리 확장 (array struct, string struct 등)
- [ ] 연산자 오버로딩 (인덱서 등)
- [ ] 복합 대입 연산자 (`+=`, `-=`, ...)
- [ ] for 루프 및 range 타입
- [ ] 모듈 시스템 (import/use)
- [ ] 예외 처리 (try-catch 또는 Result 패턴)
- [ ] 형변환 상세 문법
- [ ] `T*` sugar 문법
- [ ] 디버그 모드 (스택 포인터 범위 검증 등)
- [ ] struct 패딩/정렬 규칙
- [ ] 최적화 패스 (상수 접기, 죽은 코드 제거 등)

---

## 부록 A. 문법 요약 (EBNF 개략)

```ebnf
program     = { struct_decl | fn_decl } ;

struct_decl = "struct" IDENT "{" struct_body "}" ;
struct_body = { field_decl | method_decl } ;
field_decl  = IDENT ":" type "," ;
method_decl = "fn" IDENT "(" [param_list] ")" ["->" type] block ;

fn_decl     = "fn" IDENT "(" [param_list] ")" ["->" type] block ;
param_list  = param { "," param } ;
param       = "self" [":" type]
            | IDENT ":" type ;

type        = "int" | "char" | "void"
            | "ptr" "<" type ">"
            | "span" "<" type ">"
            | "readonly" type
            | "fn" "(" [type_list] ")" ["->" type]
            | IDENT ;

block       = "{" { statement } "}" ;
statement   = var_decl ";"
            | const_decl ";"
            | expr ";"
            | return_stmt ";"
            | if_stmt
            | while_stmt
            | "break" ";"
            | "continue" ";" ;

var_decl    = "var" IDENT [":" type] "=" expr ;
const_decl  = "const" IDENT ":" type "=" expr ;
return_stmt = "return" [expr] ;
if_stmt     = "if" expr block { "else" "if" expr block } ["else" block] ;
while_stmt  = "while" expr block ;

expr        = assignment ;
assignment  = unary "=" assignment | logic_or ;
logic_or    = logic_and { "||" logic_and } ;
logic_and   = bit_or { "&&" bit_or } ;
bit_or      = bit_xor { "|" bit_xor } ;
bit_xor     = bit_and { "^" bit_and } ;
bit_and     = equality { "&" equality } ;
equality    = comparison { ("==" | "!=") comparison } ;
comparison  = shift { ("<" | "<=" | ">" | ">=") shift } ;
shift       = addition { ("<<" | ">>") addition } ;
addition    = multiply { ("+" | "-") multiply } ;
multiply    = unary { ("*" | "/" | "%") unary } ;
unary       = ("!" | "~" | "-" | "*" | "&") unary | postfix ;
postfix     = primary { "[" expr "]" | "->" IDENT | "." IDENT
                       | "::" IDENT "(" [arg_list] ")"
                       | "(" [arg_list] ")" } ;
primary     = INT_LIT | CHAR_LIT | STRING_LIT
            | IDENT
            | type_keyword "::" IDENT "(" [arg_list] ")"
            | "std" "::" IDENT "(" [arg_list] ")"
            | "(" expr ")"
            | IDENT "{" field_init_list "}"
            | "alloc" "<" type ">" "(" expr ")"
            | "free" ["<" type ">"] "(" expr ")" ;

type_keyword = "span" | "ptr" | "int" | "char" ;

field_init_list = field_init { "," field_init } [","] ;
field_init      = IDENT ":" expr ;
```

---

*Minuto Language Specification v0.1 — Draft (revised)*
