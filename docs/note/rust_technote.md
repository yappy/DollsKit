# Rust Tech Note

## 型変換

### プリミティブ型

プリミティブ型同士の変換は型キャスト `as` を使う。
Undefined Behavior は存在しない
(Rust 1.45 で浮動小数点数の整数キャストから UB が消えたらしい。
ただしチェックの分遅くなっているので unsafe 関数が追加されている)。

### その他の型変換

`From<T>` と `Into<T>` トレイトが標準ライブラリ中にあり、多数の型パラメータに
対して実装されている。
プリミティブ型にも結局実装されている。
変換できそうな雰囲気のものは大体変換できるし、自前で実装することも簡単。
`From<T>` を実装すると勝手に `Into<T>` も実装され、そのままにするのが推奨である。
失敗する可能性のあるものは `Result` を返す
`TryFrom<T>` と `TryInto<T>` になっている。

`into()` は返り値がジェネリック型なので何らかの手段で推論させる必要がある。
構造体の初期化や関数の引数で String が必要なのに &str を渡してエラーになった
場合など、型が合わない場合は適当に `into()` パなせばある程度は OK。
文字列の場合は後述の `to_string()` でもよい。
あと `to_owned()` もある。

```rust
let s1: String = "abc".into();
```

### 文字列との型変換

文字列との変換は特別にトレイトが用意されている。

`ToString` トレイトを実装した型は、文字列 (`String`) へ変換できる。
ただし `std::fmt::Display` を実装すると自動的に `ToString` も実装されるため、
そちらが推奨されている。
そうすると `to_string()` だけでなく、
`println!()` 等のフォーマッタの対象にできるようにもなる。

`FromStr` トレイトを実装した型は、文字列 (`&str`) から変換できる。
文字列からの変換には慣用的に `str` の `parse()` メソッドを使い、
内部で `FromStr` の実装が使われる (`FromStr` を実装していれば `parse()` が
使えるようになる)。
失敗する可能性があるため返り値は `Result` になり、また返り値がジェネリックとなるため
何らかの手段で推論させるか型パラメータを自分で指定する必要がある。

```rust
let parsed: i32 = "5".parse().unwrap();
let turbo_parsed = "10".parse::<i32>().unwrap();
```

ちなみに JSON ライブラリ (serde_json) も似た雰囲気。

名前が全然違う型や関数が密接に関連していてあまり好ましく思えないが、
そういうものなので受け入れること。

### for とイテレータ

for は C とは違って完全にイテレータ限定。

`Vec` とかを for に渡すと本体がイテレータの中に move されて以後使えなくなる。
初心者狩り。
本当にこの初心者狩りが必要なものなのかは理解できていない。

```rust
for x in v {
    // ...
}
// これ以降 v を使うと既に move されておりエラー
```

(詳細) `Vec` は `IntoIterator` を実装しており、self を消費して Iterator に
自動変換されるため。

v を消費したいことはほぼ無いと思われるので、以下でよい。

```rust
for x in &v {
    // ...
}
```

```rust
for x in &mut v {
    // ...
}
```

それぞれ `v.iter()` と `v.iter_mut()` に対応している。

### イテレータとの変換

`IntoIterator` トレイトの `into_iter()` は自身をイテレータに変換する。
これを実装していると for 文の対象にできるようになる。
なお self を move して消費してしまうので以後使えなくなるという初心者が絶対引っかかる
罠がある。どうかと思うがきっと深い理由があるに違いない…。

<https://doc.rust-lang.org/std/iter/index.html#for-loops-and-intoiterator>

ほとんどのデータ構造 (コレクション) 型は `iter()` `iter_mut()` で参照を
イテレートするイテレータを返すので、それを for で回せば OK。
また、それができる場合、基本的に `impl Iterator for &Vec`
`impl Iterator for &mut Vec` のようにコンテナの参照や可変参照型に対して
`IntoIterator` が実装されており、`iter()` `iter_mut()` を返すように実装されている。
こちらの方が文字数が少なくて楽？かもしれない。

```rust
let mut values = vec![41];
for x in &mut values { // same as `values.iter_mut()`
    *x += 1;
}
for x in &values { // same as `values.iter()`
    assert_eq!(*x, 42);
}
```

<https://doc.rust-lang.org/std/iter/index.html#iterating-by-reference>

`FromIterator` は `from_iter()` で逆にイテレータからその型に変換できることを表す。
`from_iter()` の直接利用は非推奨で、これまた名前が全然違って分かりにくいが、
イテレータの `collect()` メソッドでその型に変換できるようになる。
例によって返り値がジェネリックなので何らかの方法で推論させる。

要は `Vec` の for での回し方と `collect()` での `Vec` の作り方を覚えれば、
`IntoIterator` `FromIterator` の型名を見かけたら同じような使い方ができると
理解して OK。

## データ構造

### スライス

型は `&[T]` と書く。

配列っぽいもの (メモリ上に連続して置かれているもの) (への一部)への参照。
要は `(ptr, len)`。
参照先がいつの間にか消えていてアクセス違反を起こすようなコードはコンパイラが
エラーにして弾いてくれる。
一部を抜き出したい場合にコピーが発生せず高効率で安全性も担保されているが、
~~ぶっちゃけそういうケースは少なめで~~、
配列全体を渡したい場合が多く、それもスライスの一種として渡すことになる。

関数が配列っぽいものを読み取り専用で受け取りたい時は &Vec ではなく
これで受ければ大体 OK。
配列っぽいものなら型を問わずに渡せるようになるし、
配列っぽいものの一部のみを渡せるようにもなる。

書き込み用に渡す場合は無理なので普通に `&mut Vec<T>` とか `&mut String` を渡す。
ただし空の受け取り領域を渡すくらいなら move return すれば OK。

### 文字列スライス

型は `&str` と書く。

スライスのうち文字列に特化したもの。
u8 のバイト列スライスに、UTF-8 として正しい部分列であることの保証が追加された感じ。
文字列特有の操作関数もたくさんある。

文字列を読み取り専用で受け取りたい場合はこれで受け取れば
`&str` (リテラル) と `String` の両方に対応できる。

文字列リテラル `"こういうの"` の型は `&'static str`。
`'static` はプログラムの実行中ずっと有効であるライフタイムで、
省略可能な場合も多いのであまり気にしなくてよい。

要は C/C++ と同じく文字列リテラルは実行可能ファイルの `.rodata` 領域に
置かれていて変更不能であり、
プログラムのロード時にメモリにコピーされプログラムの実行中はずっと有効であり、
文字列リテラルからは `(ptr, len)` という値が得られるということ。

文字コード問題に本当に真摯に取り組んでいる。
他の言語も頑張って。

### 配列

`[u8; 10]` のように書く。

固定長配列。
スタックに置かれるのであまり大きくしたくない。
大きな配列は凝った初期化がしにくく、ヒープに置くなら可変長の利点が要らなくても
Vec でよい感。

#### 配列定数のサイズを推論させたい

```rust
// [i32; 3]
let array = [1, 2, 3];
```

通常は型もサイズも勝手に推論してくれるが、const や static 等
型推論が使えないところでは使えない。

また、型だけ明示してサイズを推論に任せることができない。
このくらいできてもいいような気がするけど、`generic_arg_infer` は
まだ安定化されていない。。

```rust
#![feature(generic_arg_infer)]
let _arr: [f32; _] = [0.0, 1.0, 2.0];
```

現実的には、スライスを定数で初期化するのが楽。
微妙に元から意味が変わっている気がしなくもないが、
スライスは (先頭アドレス, サイズ) の組であり、そのスライスをコンパイル時定数に
しているのでサイズもコンパイル時定数にできている気がする。

```rust
const FOO: &[f32] = &[0.0, 1.0, 2.0];
```

### Vec

ヒープに置く安心と信頼の普通の可変長配列。
だいたいこれで OK。
move の勉強もこれで。

### String

文字列に特化した Vec みたいなもん。
`Vec<u8>>` に UTF-8 として不正な状態を許さないチェックを追加した感じ。
スライスは `&str` になる。

### その他データ構造

Vec 以外は use が必要。

* Vec
  * 普通の配列。
    スタックはこれで OK。
* VecDeque
  * 配列 (Vec) ベースのリングバッファ。
    キューが欲しい時は大体これで OK。
    リングバッファなのでスライスが最大2つに分かれる可能性がある。
    分かれないように並べ替えることも可能だが。
* LinkedList
  * あんまり使わない。
    アロケータみたいなものを作る時には便利な構造だがこういうヒープで作るのは
    余計に出番が少ない気がする…。
* HashMap, HashSet
  * 安心と信頼のハッシュマップとハッシュセット。
* BTreeMap, BTreeSet
  * 要は二分探索木のマップとセット。
    ソート済みの小さな配列で二分探索木を効率化した感じ。
    二分探索木として使って OK。
* BinaryHeap
  * プライオリティキューくんいたんだ…。
    概念ではなく内部実装を名前に付ける方針らしい？

## 引数を trait で受ける

以下の受け方を覚えることで受けられる型の種類が多少増えるが、

* `&Vec<T>` でなく `&[T]`
* `&String` でなく `&str`

`println!()` での `Display` や for の `IntoIterator` `Iterator<Item=T>` など、
trait を受け取る標準ライブラリや言語機能を参考にすれば受け取れる型の範囲を
増やすことができる。

### 文字列に変換できるもの全般を受ける

`to_string()` の使える型は `ToString` trait を実装している。
(`Display` trait を実装し、自動でそれを使って `ToString` が
実装されるようになっている)

<https://doc.rust-lang.org/std/string/trait.ToString.html>

```rust
pub trait ToString {
    // Required method
    fn to_string(&self) -> String;
}
```

この trait をパラメータとして受ければ文字列に変換できるもの全般を
受け方は、基本的にはジェネリックでコンパイル時解決する。
vtable による実行時解決は必要に迫られない限り推奨されない。

具体的な型ではなく抽象的なインタフェースに対してプログラミングすると柔軟性が増すという
教えは素晴らしいものだが、それをコンパイル時解決することにより実行時コストが減るので
さらに素晴らしいというのは C++ テンプレートの教えなので賛否両論。
~~ビルド遅いんですけど。~~
~~あと具体的な型1つにつき関数が1回インスタンシエートされるので命令サイズ増えますよね。~~

```rust
fn my_func1<T>(value: &T) where T: ToString {
    let s = value.to_string();
}
```

* `my_func1` を1つの型パラメータ `<T>` をとるジェネリック関数として定義する。
* 渡された値の型を `&T` で受けて、T に型を当てはめさせる。
* where 句で `T` は `trait ToString` を実装しているという制約を宣言する。
  これで `T` は `impl ToString` していることが保証されるようになるため、
  `value.to_string()` 呼び出しのコンパイルが通るようになる。
* where は数学英語で、"ここで、" "ただし、" のような条件や制約を示す。
  * y=ax+b where a, b are integer constants.
  * y=ax+b ただしここで a, b は整数の定数。
  * 英語話者優遇文法を許すな。
* よって以下のように読めばよい。
  * `my_func1` は型パラメータ `<T>` を持つ。
  * 引数は `value: &T`、つまり `value` は `T` への不変参照型である。
  * ただしここで型パラメータ `T` は `impl ToString` しているものに限る。
  * すなわち `T` 型は `to_string()` メソッドを利用可能である。

```rust
fn my_func2<T: ToString>(value: &T) {
    let s = value.to_string();
}
```

慣れてきたらこのように省略できる。

```rust
fn my_func3(value: &impl T) {
    let s = value.to_string();
}
```

さらに慣れればここまで省略できる。
ただし型パラメータが無くなったように見えるが impl はシンタックスシュガーなので
やはりジェネリック関数である。

ここまで言っておいてなんだが `to_string()` は常に新しい `String` オブジェクトを
生成してしまう (ヒープに malloc する) のでそれでいい場合以外は微妙。

文字列参照に変換できる型全般を受け取りたい場合は `AsRef<str>` だが、
`String` と `&str` を両方受けたいなら `&str` スライスで受けて
`String` の時は `&string_variable` で渡すだけなのでジェネリックで受ける意味は
あまりない…。

```rust
pub trait AsRef<T>
where
    T: ?Sized,
{
    // Required method
    fn as_ref(&self) -> &T;
}
```

もうちょっと高度なものに `Cow` (copy-on-write と見せかけて clone-on-write)
trait がある。
無理して使うようなものではないと思うが、ライブラリが使っているとよく分からなくて
きれそうになるので…。
これは参照による借用とそのものを所有するケースを enum で両対応したものである。

```rust
pub enum Cow<'a, B>
where
    B: 'a + ToOwned + ?Sized,
{
    Borrowed(&'a B),
    Owned(<B as ToOwned>::Owned),
}
```

内部が知らない trait とライフタイムでいっぱいできれそうになるが、
`B` に `str` を入れてみると `Borrowed(&str)` となる。
読み取りに使っている間はそのまま低コストな `&str` で取り扱い、
書き込みが必要になったらその時に `to_owned()` を呼び出して `String` を生成する。

`ToOwned` ってなんやねんという感じだが、`Clone` をもう少し一般化したものである。
`Clone` は不変参照 `&T` からコピーオブジェクト `T` への変換だが、
`&str` と `String` は全然別の型なので、別の型に対しても適用できるようにした
バージョンという感じである。
`B` を `str` とすると、`<B as ToOwned>::Owned` は `String` に解決される。

最終的な結論としては `Into<Cow<'a, str>>>` で受けるのが一種のイディオムになっている
ようだが、そこまですべきなのかはよく分からない。
クローンせず不変参照だけで処理完了するケースがそれなりに存在するなら価値はありそうだが、
重いクローン処理するかどうかが実行時解決になりコードサイズ
~~とコンパイル時間~~が大きくなる気がする。

## スマートポインタ

一般的な参照管理のツール類。

### Box

あまりに普通過ぎて逆に使わない。かも。
C++ unique_ptr 相当に思う。

* 単にデータをスタックではなくヒープに置きたい。
  * クソデカ配列や構造体をローカル変数に取ったら clippy に警告された。
    * 配列の場合は Vec を使っちゃえばよさそう。。
* コンパイル時にサイズが決定できないためスタックに置けなかった。
  * trait (vtable) によるポリモーフィズムを実行時に行いたい場合。
    * 引数に取る場合は ~~テンプレート~~ ジェネリクス使用でコンパイル時に決定できるが、
      trait を返したい時はこちらで。
    * この用法では vtable 使用の実行時コストを明示するため、
      `Box<dyn MyTrait>` のように `dyn` キーワードが必要。
* 実データでリンクリストを作ろうとしたら無限再帰になってしまうのでポインタにしたい
  (Rust book の例)。

### Rc

Reference Count。参照カウント。
所有権を複数人で共有し、被参照数を表す単一の整数で管理する。
誰からも参照されなくなった瞬間に1回だけ解放される。

C++ shared_ptr の参照カウンタ操作部分がスレッドセーフでないもの。
スレッドセーフにして shared_ptr と同等にしたものが `Arc`。

C++ も古くは auto_ptr が vector 等のコンテナに入れられないため
スレッドセーフな shared_ptr を作って入れていた時期があったが、
普通にオーバースペックである。
まずスレッドセーフでなくていいし、参照カウントもいらない
(所有権のみでよい => unique_ptr, Box)。

スレッドセーフでない Rc は実戦では使いどころが少ない気がする。

循環参照の問題や、弱参照も一般的な考え方の通り。
Rust を使っていると全ての安全性をコンパイラが保証してくれる錯覚に陥るが、
そんなことはないので注意。

### Arc

Atomic Reference Count。
参照カウントにアトミック変数を用いたもの。
カウンタのインクメント、(デクリメント + 0 になったかの確認) にアトミック命令を
使うことで、複数スレッド間でのオブジェクトの共有を可能にする。
全ての所有者が所有権を放棄した瞬間に1回だけ解放される。

マルチスレッドプログラムを書くととんでもなく出番が多い。
全てのスレッドから参照されなくなった時点で自動的に1回だけ解放できる。
長生きするはずの親スレッドがエラーで早期終了した場合等にも対応できて大変便利。

C で書かれた OS カーネルでも多用されるしそれはそう。

## マーカートレイト

### Send

他のスレッドに move できることを示す。
ほとんどの基本型は Sync で、内容が全て Sync なら複合型も自動的に Sync になる。

特別な事情がない限りはほとんどのデータはスレッドをまたいでも問題ないが、
例えば Rc は clone して作ったもう1つの参照を別スレッドに転送してしまうと
2つのスレッドから1つの参照カウントをノンアトミックに操作することになり、
データレースを起こして壊れてしまうため、Send がついていない。

### Sync

&T が Send である型。
正確な定義は難しいので、簡単に言うと複数スレッドから参照され
同時にアクセスしても安全な型。

こちらは普通の型はそんなことはない。
`Mutex` とかに包むと Sync にできる。
名前からしても同期が取れているという意味で、分かりやすい。

## 内部可変性

大体わかった、とプログラムを書いてみると、Rc や Arc の中身が変更できなくて詰む。

<https://doc.rust-lang.org/std/cell/>

Rust の borrow checker は、1つのオブジェクトに対して

* 1つ以上の不変参照 `&T`
* 1つの可変参照 `&mut T`

のどちらか **のみ** を許可することによってメモリ安全性をコンパイル時に保証している。
shared-exclusive lock のルールと同じ。

1つのオブジェクトの所有者を `Rc` や `Arc` で増やした場合、
安全性を保証するには中身を immutable にするしかない
(2つ以上の可変参照を同時に作れるようになってしまうため)。
しかしそれだと可変なオブジェクトを共有することができなくなってしまい、
まともにプログラムを書けなくなってしまう。

自身が immutable だとしても中身を mutable として変更できるのが `std::cell` シリーズ
(シングルスレッド限定)。
マルチスレッドに対応したのが `Mutex`, `RwLock` など。また、atomic も。
強力なコンパイル時チェックを一部緩和することができる。

当然ただ緩和するだけだと壊れる可能性があるので、何らかの対応が取られている。

### Cell, RefCell

自身が immutable でも、中身の値を変更できるのが `Cell`。
内部への参照は禁止され、値の取得と設定は全て move で行う必要がある。
Copy ならば move がコピーになるため、扱いやすい。
もしこれでうまくいくならば、こちらを使う。

そのような制約が無いのが `RefCell`。
const でない値への const なポインタみたいなもの。
borrow check を実行時に移動する。なので実行時オーバーヘッドはある。
不変・可変リファレンスカウントがなされ、ルールを破ると実行時に panic する。

### Mutex, RwLock

スレッドセーフ版。
中身を可変にする効果もついてくる。

どちらもロックによってマルチスレッド環境で一度にアクセスできるスレッドを
borrow chcker の行うのと同じルールで実行時に制限できている。

### 結論 (テンプレ)

```rust
Rc<RefCell>
Arc<Mutex>
Arc<RwLock>
```

## グローバル変数

コンパイル時定数ならばかなり気軽に使えるが、
グローバル変数となると当然スレッドセーフの保証などが絡むため、
コンパイラに許してもらうのがややこしくなる。
先に内部可変性の話を読むこと。

お行儀のよいプログラムだと出てこないため、入門書を読んだ直後に困りがち。
~~使うな。~~

### const 定数

型は省略できない。
グローバルスコープで宣言可能。
ローカルスコープで宣言することもできる。

単純な定数はだいたいこれで OK。

```rust
const LANGUAGE: &str = "Rust";
const THRESHOLD: i32 = 10;
```

* immutable
* メモリアドレスを持たない (使用するとインライン展開される)
* ライフタイムは 'static (プログラムの実行中はずっと有効)

### static 変数・定数

const と同じ文法だが、こちらは mut にすることもできる。
しかしながら普通の書き込み可能グローバル変数はスレッドセーフ性ゼロなので、
unsafe を使わないと普通の型に対してはほとんど役に立たない。

```rust
// 注: safe Rust でははっきり言って意味のないコード
static mut THRESHOLD: i32 = 10;

fn main() {
  // unsafe で括らないとコンパイルエラーを起こす
  THRESHOLD = 20;
}
```

* mutable (static mut) 宣言も可能 (なお使用は unsafe)
* メモリアドレスを持つ
  * 多分 .data or .rodata に置かれる？
  * immutable 宣言で const と違ってメモリアドレスを持つ定数を作れるが、
    const も使用時に一時領域に置かれてアドレスが取れるので有効性はやや不明。
    C/C++ にずっとアクセスできる定数領域を渡す時とか？
* ライフタイムは 'static (プログラムの実行中はずっと有効)

### const / static 変数のスコープ

グローバル変数と書いてしまったが、関数内に書くこともできる。
1つの関数からしかアクセスしないのならばこれで OK。
struct の中や impl の中でも可能らしい。
なるべく可能な限り内側に書こう。

C の static ローカル変数 (静的記憶クラス) に近い。
C がファイル内リンケージのグローバル変数・関数 (C++ 無名 namespace 相当) に
同じキーワード static を使っているのはよくないことだとされている。

* ファイル内のみからアクセス可能
  * C - static グローバル関数・変数
  * Rust - `pub` のない関数・const/static 変数
* 生存期間をプログラム実行中全域とする (.data/.rodata/.bss に置く)
  * C - static ローカル変数、グローバル変数は自動的にそうなる
  * Rust - static グローバル変数、static ローカル変数
    * const はコンパイル時定数のため、メモリには置かれない。
    * ライフタイムを持たないし、ポインタを取得することもできないが、
      適当なローカル変数をその定数で初期化すればほとんどの場合解決する。
    * というわけで mut でない static は出番はほとんど無い。

### 普通に書き込めるグローバル変数

なぜ static mut が unsafe かというと全てのスレッドの全ての場所から
書き込み可能というのはどう考えても Rust の保証する参照の制限を最初から破っているから。

mut でない static で内部を変更可能にするため Cell を使ってみると、
スレッドセーフでないと怒られる(当たり前)。
スレッドセーフ性は `Sync` マーカートレイトで表される。

```rust
static C: Cell<u32> = Cell::new(1);

/*
12 | static C: Cell<u32> = Cell::new(1);
   |           ^^^^^^^^^ `Cell<u32>` cannot be shared between threads safely
   |
   = help: the trait `Sync` is not implemented for `Cell<u32>`
   = note: if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` or `std::sync::atomic::AtomicU32` instead
   = note: shared static variables must have a type that implements `Sync`
*/
```

親切にメッセージに出ているが、スレッドセーフな内部可変セル
(`Mutex`, `RwLock`, atomic など) を immutable static 宣言するのが正解。

```rust
static GLOBAL: Mutex<MyData> = Mutex::new(...);
```

Mutex で包めばスレッドセーフで書き換え可能なグローバル変数を実現できるということで、
直感にも合っている。

### 複雑な初期化

const や static は宣言時に初期化する必要があり、関数呼び出しは const fn
(コンパイル時に決定できる値、C++ constexpr みたいなもの) でなければならない。

* const - コンパイル時に使用箇所を置き換えるため
* static -  `.data` や `.rodata`、`.bss` に置くために初期値が
  コンパイル時に決定できる必要がある。

1回だけ値を変更でき、以後変更不能になるセルとして `OnceCell` と `OnceLock` がある。
`OnceLock` は `OnceCell` のスレッドセーフ版で、static に使うならスレッドセーフ性が
必要。

#### 初期化が複雑なグローバル定数

変更されない HashMap とか？

以下 OnceLock の公式サンプル

```rust
fn computation() -> &'static DeepThought {
    // n.b. static items do not call [`Drop`] on program termination, so if
    // [`DeepThought`] impls Drop, that will not be used for this instance.
    static COMPUTATION: OnceLock<DeepThought> = OnceLock::new();
    COMPUTATION.get_or_init(|| DeepThought::new())
}
```

static 宣言時には OnceLock::new() で空の状態で初期化すればよい。
`get_or_init()` はセルが空の場合、引数の関数を呼び出してその返り値でセルを初期化し、
その値を返す。
既に初期化されていた場合、その値を返す。

`get_or_init()` には毎回初期化されていなかった時のための関数の指定が必要なので、
`static OnceLock` をグローバルスコープに置くのではなく、
サンプルのように関数のローカルスコープに置いて、
`get_or_init()` の返り値をグローバル関数として公開するのがよさそう。

`OnceLock` はこれをスレッドセーフに行う。

最初の呼び出しだけ重くなるのが嫌なら、プログラムの最初で `set().unwrap()` して、
取得時は `get().unwrap()` すればいい気がする。

#### 初期化が複雑なグローバル変数

`OnceLock` は最初の一回だけ初期化を呼ぶためのもので、読み書き可能グローバル変数は
素直に `Mutex` や `RwLock` でよい気がする。

## エラーハンドリング

関数の返り値を Result にするとき、`Err<T>` 時の型が合っていないと
? 演算子で楽ができない。
このままだと関数内でいろいろな種類のエラーが返ってくる場合に困る。
そこで、標準ライブラリのエラーは `std::error::Error` トレイトを実装しているので
この型でエラーを返すようにする。

The Book でも実は
<https://doc.rust-jp.rs/book-ja/ch12-03-improving-error-handling-and-modularity.html>
で導入している。
(Result のエラーハンドリングの章にはない。。)

Rust By Example ではここ。
<https://doc.rust-jp.rs/rust-by-example-ja/error/multiple_error_types/boxing_errors.html>

オススメ

```rust
Result<(), Box<dyn Error + Send + Sync + 'static>>
```

コンパイル時に型を決定できない方法になるので Box によるヒープポインタ参照を使う。

Send, Sync 制約をつけるとスレッド間でエラーオブジェクトを受け渡せるようになる。
別スレッドで実行した結果を Result として受け取れるようになる。

'static をつけると `is` や `downcast_ref` 等が使えるようになる。

<https://doc.rust-lang.org/stable/std/error/trait.Error.html#method.is>

## エラーハンドリング - anyhow

標準ライブラリのみで行うならば前節の通り。

<https://qiita.com/legokichi/items/d4819f7d464c0d2ce2b8>
スマートなエラー処理に関しては昔からかなり苦労しているらしい…。

anyhow はこの辺りをもっと簡単に書けるようにしてくれるライブラリ。
2019 年頃登場し、2022 年現在デファクトと言える。
特に標準ライブラリのみで頑張る必要がないのであれば、ほぼすべてのプロジェクトの開始時に
依存を張ってよいタイプのライブラリと言えそう。
(個人的にはあまりそういうのは好きではないけど、Rust は基本的なところですら変遷期であり、
しばらく時間が経てば標準入りするんじゃないか？)

<https://docs.rs/anyhow/latest/anyhow/>

使い方は、とりあえず全部 `anyhow::Result<T>` を返せば OK!
これは `Result<T, anyhow::Error>` に等しい。
これで `std::error::Error` を返せるようになるので、普通のエラーなら何でも
? 演算子で返せるようになる。
(`From` trait によって実現されているっぽい。)

`std::process::Termination` を実装しているので main から返しても大丈夫。

```rust
fn main() -> anyhow::Result<()> {
    Ok(())
}
```

Send + Sync 問題とかも解決しているので async fn から返しても大丈夫。

ダウンキャストの確認や実行も可能。

`context()` を使うと情報を付け加えつつエラーを投げられる。

```rust
fn main() -> Result<()> {
    ...
    it.detach().context("Failed to detach the important thing")?;

    let content = std::fs::read(path)
        .with_context(|| format!("Failed to read instrs from {}", path))?;
    ...
}
```

nightly channel を使うか features "backtrace" を指定するとバックトレース機能が
使えるようになる。(そのうち安定化して標準化するかも？)

`anyhow::Error` を作りたい場合には `anyhow!` マクロが便利。
`format!` と同じ用法でエラーを作れる。

`bail!` は `return Err(anyhow!(...))` でより簡単に early return できる。
`ensure!` は `ensure!(user == 0, "only user 0 is allowed");` のように
if まで省略して early return できる。

## panic

通常のハンドル可能なエラーは `Result` で返す。

そうでない深刻なエラーは `panic()` を呼ぶ。
`unwrap()` や `assert!()` など、失敗時に panic させる便利関数も多い。

panic は要は C++ 例外で、通常のエラーは C スタイルの整数リターンと早期 return で
十分で、通常のエラーを C++ 例外で返すのは重いという至極真っ当な指摘に基づくと思われる。
panic はスレッドごとに処理される。

~~何でもかんでもエラーは例外で返そうとする C++ 標準ライブラリと~~
~~整数リターンと ? 演算子による eraly return のシンタックスシュガーでよくね？~~
~~と主張する Rust、どこで差がついたのか~~

ちなみに Rust は LLVM の libunwind (元々 C++ 用でもある) を使っているらしい。

panic 関連は `std::panic` モジュールにあり、パニックハンドラを設定できたりするが、
有効な用途は少ないかもしれない。
一応 catch もできる。

<https://doc.rust-lang.org/std/panic/index.html>

組み込みやベアメタル環境では panic 周りに手を入れる必要があるかも。

### オプション

デフォルトではそのスレッドのみが異常終了するが、大多数のアプリケーションでは
プロセスごと強制終了する方が自然ということでオプションが追加された。
デフォルトではないので注意。

unwind する必要が無ければ色々なオーバーヘッドを少なくできる気がするが、
実際のところはよく分からない。

* `-C panic=unwind` on rustc, `panic = 'unwind'` on Cargo.toml
  * デフォルトの動作。スタックアンワインドを行いスレッドを強制終了させる。
  * マルチスレッドプログラムの場合、生き残った他のスレッドが動き続けてプログラムは
    強制終了しないので、罠っぽい動作になるので注意。
* `-C panic=abort` on rustc, `panic = 'abort'` on Cargo.toml
  * プログラムをその場で異常終了させる。

### Mutex のポイゾニング

スレッド間共有データのロック中にそのスレッドが panic した場合、
スタックアンワインドによって `drop()` が呼ばれアンロックはされるが、
共有データの不変条件が壊れている可能性がある。
その場合、そのロックは poisened 状態としてマークされ、ロックすると失敗する
(`Result` で `Err` が返る)。

`panic=abort` 設定の場合は気にする必要がないので `unwrap()` してしまってよいだろう。

## Trait

要はインタフェース。

### 定義

struct と同じような気持ちで書けば OK。
ただしメンバはフィールドではなく関数を書く。
関数の実装は書かないのでセミコロンで終わらせる。
ただしデフォルト実装を書くこともできる。

```rust
pub trait Summary {
  fn summarize_author(&self) -> String;

  fn summarize(&self) -> String {
    format!("(Read more from {}...)", self.summarize_author())
  }
}
```

### Trait を引数に取る (static dispatch)

C++ と同様 ~~テンプレート~~ ジェネリクスで受けるのを推奨。

トレイト境界の記述が長くなっても読みやすい書き方

```rust
fn notify<T>(item: &T)
  where T: Summary,
{
  println!("Breaking news! {}", item.summarize());
}
```

やや省略した書き方

```rust
fn notify<T: Summary>(item: &T) {
  println!("Breaking news! {}", item.summarize());
}
```

最も省略した書き方 (シンタックスシュガー多すぎない？覚えられないんだが)

```rust
fn notify(item: &impl Summary) {
  println!("Breaking news! {}", item.summarize());
}
```

引数は `&T` 型で、ただしここで (where の数学用語としての意) `T` は
trait `Summary` を実装している、という意味。
1つの型パラメータに複数のトレイト境界を指定したい場合は `+` でつなぐ。

`Summary` を実装した struct を渡せば `T` がその型に推論され、
`some_function<ThatType>` がインスタンス化される。
呼び出した型の種類数分の関数が生成されプログラムサイズが増加するが、
コンパイル時に呼び出し関数が決定されるため、
通常の関数呼び出しと実行時コストに差は無い。

~~こういうことをするから C++ と同じくらいコンパイルが遅い。~~

### Trait を引数に取る (dynamic dispatch)

基本的には推奨されないが、vtable の関数ポインタによる動的ディスパッチもできる。
C++ の virtual 関数と同様。
関数ポインタ経由の呼び出しになるため実行時オーバーヘッドはある。
`self` を引数に取らない関数が NG になったり、
`self` の move 渡しが不可能になったりする。
一応、コードサイズが肥大化しないというメリットはある。

元々は `&MyTrait` のように書いていたようだが、
型名とトレイト名を混同した書き方はよろしくない
(型は値の分類で、トレイトは型の分類であり、根本的に別物である)
ということと、dynamic dispatch のオーバーヘッドがあるということを明示的に
表記するために文法が変わったようである。
猶予期間を経て現在は dyn が無いとコンパイルエラーになり、dyn をつけろと言われる。

```rust
fn notify2(item: &dyn Summary) {
  println!("Breaking news! {}", item.summarize());
}

// 古い書き方 (エラーになり dyn をつけろと言われる)
// fn notify2(item: &Summary) 
```

### Trait を返す・保存する

コンパイル時に決まらない型を扱う場合、型のサイズが決定できないので
スタック上に置くことはできない。
`Box` を使ってヒープに動的確保し、ポインタで管理する。
色々な異なる型を trait の Vec として管理する場合も同様。

```rust
fn create() -> Box<dyn Summary> {
  // ...
}
```

### Trait を返す (コンパイル時に1種類に限定できる場合)

trait (イテレータやクロージャなども) を返す場合、型がコンパイル時に決定できる
(返す可能性のある具象型が1種類) ならば、`impl Trait` 構文が使える。
この場合 `Box` を使わなくて済むし、dynamic dispatch も回避できる。

```rust
fn create() -> impl Summary {
  // ...
}
```

## 並列テストが失敗する

デフォルトで test attribute のついたテストは並列実行される。
ファイルやグローバル変数等のグローバル状態を変更するテストは
並列実行すると失敗する可能性がある。

`cargo test` に `--test-threads=1` をつけるとシングルスレッド実行になるが、
並列化可能なところまで直列化されてしまう。

`mod test` 内にグローバル変数として Mutex を用意し直列化したいテストで
ロックを取れば直列化できるが、assert 失敗で panic した場合に
Mutex の PoisonError で他のテストを巻き込んで失敗してしまう。

<https://github.com/rust-lang/rust/issues/43155>

結論としては `serial_test` クレートを使うのが便利。

```rust
// これがついたテストは直列化される
#[serial]
// 引数をつけるとその名前のグループ内でのみ排他される
#[serial(group)]
// serial とは排他されるが、parallel 同士は同時に実行可能
#[parallel]
// こちらもグルーピング可能
#[parallel(group)]
```
