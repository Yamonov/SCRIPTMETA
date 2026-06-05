# SCRIPTMETA v1.4 仕様

## 1. 目的

**SCRIPTMETA** は、スクリプト本体や配布ページに書ける軽量なメタデータ形式です。

主な用途は次の2つです。

- スクリプトの更新確認
- ローカルのスクリプト一覧で説明文などを表示すること

Adobe アプリ用の jsx などのスクリプトは、配布ページと手元のスクリプトの結びつきが弱く、配布後に不具合修正や更新があっても利用者が気づきにくいことがあります。SCRIPTMETA は、スクリプト本体に識別子、バージョン、参照先を埋め込み、クライアント側で更新を検知できるようにします。

SCRIPTMETA は **自動ダウンロードや自動インストールを目的としません**。更新検知後の取得と導入は、利用者が配布ページを確認して手動で行います。

## 2. 2つの使い方

SCRIPTMETA v1.4 では、用途を分かりやすくするために次の2つを定義します。

- **Update Profile**
- **Local Subset**

どちらも script 側では同じタグを使います。

```text
SCRIPTMETA-BEGIN
...
SCRIPTMETA-END
```

別タグを増やすわけではありません。必須項目と用途が違います。

## 3. Update Profile

Update Profile は、更新確認に使う正式な構成です。

### 3.1 script 側

必須項目:

- `Script-ID`
- `Version`
- `Meta-URL`

推奨項目:

- `Description`
- `Target-App`
- `Name`
- `Author`

任意項目:

- `Edit-Password-SHA256`

条件付き必須項目:

- `Edit-Password-SHA256` を使う場合は `Author`

例:jsxコードの先頭にブロックコメントで挿入した例

```text
/*
SCRIPTMETA-BEGIN
Script-ID=com.yamadataro.Photoshop_InDesign_Crop
Name=Photoshop InDesign Crop
Author=Yamada Taro
Target-App=Photoshop
Version=1.2
Meta-URL=https://gist.github.com/yamadataro/d06d117b..........
Description-BEGIN
Photoshopで実行すると、InDesign上に配置した画像のフレーム枠に合わせて画像サイズを変更します。
正立のみ対応し、回転した画像には非対応です。
Description-END
SCRIPTMETA-END
*/
```

説明:

- `Script-ID` は全配布者の中で一意になることを強く目指してください。
- `Version` は手元のスクリプトと配布ページ側の最新版を比較するために使います。
- `Meta-URL` は最初に参照する SCRIPTMETA 情報ページです。
- `Description` はクライアント側で説明表示に使えます。Update Profile でも記載を推奨します。
- `Target-App` は Illustrator、Photoshop、InDesign などの対象アプリを示すために使えます。
- `Name` はクライアント側で表示名として使えます。省略した場合はファイル名など、クライアント側の通常表示名を使います。
- `Author` は作者名です。Update Profile では記載を推奨します。
- `Edit-Password-SHA256` は対応する編集UIで編集前の確認に使う任意項目です。

### 3.2 配布ページ側

配布ページ側は、script 側と別の専用タグを使います。

```text
SCRIPTMETA-DIST-BEGIN
...
SCRIPTMETA-DIST-END
```

必須項目:

- 共通: `Script-ID`
- 最新版をこのページで直接示す場合: `Version`
- 別ページへ案内する場合: `Latest-URL`

`Version` と `Latest-URL` の両方を記載しても構いません。その場合は `Latest-URL` を優先します。旧版ページで `Version` を残しても動作しますが、非推奨です。

`Latest-URL` は、次に参照すべき SCRIPTMETA 情報ページの URL です。配布ファイルそのものの URL ではありません。

配布ページ側では、更新確認に使う `Script-ID`、`Version`、`Latest-URL`、`Latest-Page-URL` だけを記述対象にします。`Name`、`Author`、`Description`、`Target-App`、`Min-Target-Version`、`Release-Date`、`Edit-Password-SHA256` は配布ページ側では解釈しません。

最新版ページの例:

```text
SCRIPTMETA-DIST-BEGIN
Script-ID=com.yamadataro.Photoshop_InDesign_Crop
Version=1.2
SCRIPTMETA-DIST-END
```

旧版ページから最新版ページへ案内する例:

```text
SCRIPTMETA-DIST-BEGIN
Script-ID=com.yamadataro.Photoshop_InDesign_Crop
Latest-URL=https://gist.github.com/yamadataro/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
SCRIPTMETA-DIST-END
```

配布ページ側の1つのブロックには、複数のスクリプト情報を記述できます。

```text
SCRIPTMETA-DIST-BEGIN

Script-ID=com.yamadataro.Photoshop_InDesign_Crop
Version=1.2

Script-ID=com.yamadataro.Photoshop_InDesign_Resize
Version=1.5.2

SCRIPTMETA-DIST-END
```

ルール:

- `Script-ID` はレコードの先頭に置きます。
- 次の `Script-ID` までを同一レコードと見なします。
- 配布ページ側の `Description` は無視します。

## 4. Local Subset

Local Subset は、ローカルのスクリプト一覧で説明文などを使うための緩い構成です。

主な目的は、更新確認ではなく、ローカル開発中や個人利用のスクリプトに説明を付けることです。

必須項目:

- `Script-ID`

推奨項目:

- `Description`
- `Version`
- `Target-App`
- `Name`

任意項目:

- `Author`
- `Edit-Password-SHA256`

条件付き必須項目:

- `Edit-Password-SHA256` を使う場合は `Author`

書かない項目:

- `Meta-URL`

Local Subset に `Meta-URL` は書かないでください。Local Subset は更新確認対象ではないため、`Meta-URL` を書いても意味が曖昧になります。`Version` と `Meta-URL` が揃うと、ソフトウェアが更新先を見に行く場合があります。更新確認したい場合は Update Profile に移行してください。

例:

```text
/*
SCRIPTMETA-BEGIN
Script-ID=com.yamadataro.superFrameFitPlus
Name=Super Frame Fit Plus
Target-App=InDesign
Version=0.1
Description-BEGIN
選択中のフレームに合わせて画像を調整します。
ローカル開発中のスクリプトです。

Descriptionブロック内のDescription-BEGINはタグではなく本文として扱われます。
BEGIN-END間は、前後の空白を除いた行全体がDescription-ENDの場合だけ閉じタグとして扱い、それ以外は改行も含めた説明文として扱われます。
■Description-END　は閉じタグと見なしません。後のタグの詳細説明も参照してください。
Description-END
SCRIPTMETA-END
*/
```

`Description` がない Local Subset も有効です。ただし、説明表示の用途では情報が出ません。ローカルで使う場合も `Description` の記載を推奨します。

### 4.1 Local Subset から Update Profile への移行

Local Subset から Update Profile へ移行する場合は、基本的には同じ `Script-ID` のまま、Update Profile で必要な項目を追加します。

```text
Version=1.0
Meta-URL=https://gist.github.com/yamadataro/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
```

Local Subset で仮の `Script-ID` を使っていた場合は、公開時に変更しても構いません。ただし、将来の移行を考えるなら、最初から公開しても困らない `Script-ID` にしておくことを推奨します。

## 5. Script-ID

`Script-ID` はスクリプトを識別する ID です。Update Profile でも Local Subset でも必須です。

全配布者の中で一意になることを強く目指してください。逆ドメイン記法に近い形式を推奨します。

Local Subset でも、将来 Update Profile へ移行できるように、最初から一意性を意識した ID にしてください。

例:

```text
Script-ID=com.yamadataro.Photoshop_InDesign_Crop
Script-ID=com.yamadataro.Photoshop_InDesign_Crop_ForPrint
Script-ID=com.yamadataro.Photoshop_InDesign_Crop_2
```

考え方:

保持しているドメインが「taro.co.jp」なら、「jp.co.taro.UniqueScriptname」
となります。

```text
com.自分の名前のローマ字表記スペース無し.スクリプトファイル名
```

必要なら末尾に連番や用途名を付け、必ず一意になるようにしてください。

使用可能文字の目安:

- `A-Z`
- `a-z`
- `0-9`
- `.`
- `_`
- `-`

### 5.1 Name

`Name` は、スクリプトの表示名です。Update Profile と Local Subset のどちらでも任意で使えます。

`Name` は人に見せるための名前で、スクリプトの識別には使いません。識別には必ず `Script-ID` を使います。

例:

```text
Name=Super Frame Fit Plus
```

クライアントは、説明表示や設定画面で `Name` を表示できます。`Name` がない場合は、ファイル名など既存の表示名を使います。

### 5.2 Author

`Author` は、スクリプトの作者名です。

Update Profile では記載を推奨します。Local Subset では任意です。

`Edit-Password-SHA256` を書く場合は、Update Profile と Local Subset のどちらでも `Author` を必須とします。編集ロックを誰が設定したか分からない状態を避けるためです。

例:

```text
Author=Yamada Taro
```

`Author` は1行のテキストとして扱います。スクリプトの識別には使わず、識別には必ず `Script-ID` を使います。

### 5.3 Edit-Password-SHA256

`Edit-Password-SHA256` は、対応する SCRIPTMETA 編集UIで、編集前にパスワード確認を行うための任意タグです。

書式:

```text
Edit-Password-SHA256=<salt>:<sha256>
```

`sha256` は、次の文字列を UTF-8 として SHA-256 にした64桁の小文字16進文字列です。

```text
salt + ":" + password
```

例:

```text
Author=Yamada Taro
Edit-Password-SHA256=salt123:41eb6f7f12752e69bb7c7a847624085953d719537fe558f7ce0105061f543a8d
```

ルール:

- `salt` は空でない ASCII 文字列を推奨します。
- `sha256` は64桁の16進文字列です。
- パスワードそのものは書かないでください。
- `Edit-Password-SHA256` を書く場合は、同じブロック内に `Author` も書いてください。
- 対応する編集UIは、`Edit-Password-SHA256` がある場合、編集画面を開く前にパスワード入力を求めることができます。
- パスワードが一致しない場合や入力をキャンセルした場合、編集画面を開かないことを推奨します。

注意:

SCRIPTMETA はプレーンテキストです。`Edit-Password-SHA256` は、本当の改ざん防止や作者証明ではありません。エディタで直接ファイルを開けば、タグを削除したり変更したりできます。このタグは、SCRIPTMETA 対応編集UIの中で、作者用の編集を不用意に行わないための制限として扱います。

`Edit-Password-SHA256` は、更新確認、配布元の正当性判定、自動ダウンロード、自動インストールには使いません。

## 6. Description

`Description` は、スクリプトの説明文を複数行で記述するためのブロックです。

script 側で使います。Update Profile と Local Subset のどちらでも記載を推奨します。

書式:

```text
Description-BEGIN
ここに説明文
複数行可
Description-END
```

ルール:

- `Description-BEGIN` と `Description-END` の間の内容を説明文として扱います。
- 改行を含めて保持します。
- 改行コードの正規化はパーサ側で行います。
- `Description-BEGIN` と `Description-END` は単独行でのみ有効です。
- 実装上は、前後の空白を除いた行全体が `Description-BEGIN` または `Description-END` の場合だけタグとして扱うことを推奨します。
- 説明文ブロック内はキー解析しません。
- 説明文ブロックは、直前の `Script-ID` のレコードに属します。
- 説明文ブロックは 1 レコードにつき 1 個までです。
- 配布ページ側の `Description` は無視します。

次の行は終端タグです。

```text
Description-END
   Description-END
Description-END
```

前後に空白があっても、空白を除いた行全体が `Description-END` であれば終端タグです。

次の行は終端タグではなく、説明文として扱います。

```text
■ Description-END
Description-END です
本文 Description-END
```

`Description-END` を本文中に書きたい場合のエスケープ形式は仕様に含めません。必要な場合は、上の例のように別の文字を付けてください。

## 7. Version

`Version` は、Update Profile の script 側では必須です。配布ページ側では、最新版を直接示すレコードで必須です。

Local Subset では推奨です。

バージョンは `.` 区切りの整数列を基本とします。

例:

```text
1
1.5
1.5.2
1.5.2.1
```

クライアント実装では、`v1.2.3` のような表記から数値部分を取り出して比較しても構いません。

## 8. Meta-URL

`Meta-URL` は、Update Profile の script 側で必須です。

そのスクリプトが最初に見に行く SCRIPTMETA 情報ページ URL を指定します。

`Meta-URL` には、note、Qiita、gist、GitHub リポジトリ、GitHub ディレクトリなどの URL を指定できます。

Local Subset では `Meta-URL` を書かないでください。

## 9. Gist と GitHub

gist を使う場合は、`SCRIPTMETA.txt` を作成してそこに配布ページ側の情報を記述してください。クライアントはこのテキストファイルを優先して探します。

GitHub リポジトリ全体を配布ページとして使う場合は、repo 直下に `SCRIPTMETA.txt` を置くことを推奨します。GitHub リポジトリでは、更新判定の正本は `SCRIPTMETA.txt` とします。Releases や README の本文は、更新判定の正本としては扱いません。

GitHub の特定ディレクトリを配布ページとして使う場合は、そのディレクトリ内に `SCRIPTMETA.txt` を置いてください。`Meta-URL` には GitHub のディレクトリ表示 URL を指定します。

例:

```text
Meta-URL=https://github.com/Yamonov/Iwashiya_Scripts/tree/main/Illustrator
```

この場合、利用者が `Meta-URL` を開くと `Illustrator` ディレクトリが表示され、そこから JSX を選択して確認やダウンロードができます。クライアントは更新判定のために、同じディレクトリ内の `SCRIPTMETA.txt` を raw テキストとして取得します。

運用サンプル:

[Yamonov/Iwashiya_Scripts](https://github.com/Yamonov/Iwashiya_Scripts/) では、リポジトリ直下をスクリプト集の入口にし、`Photoshop`、`Illustrator` などのアプリ別ディレクトリに JSX と `SCRIPTMETA.txt` を置く構成を想定します。

この構成では、script 側の `Meta-URL` はリポジトリ直下ではなく、対応するアプリ別ディレクトリを指定します。

```text
Meta-URL=https://github.com/Yamonov/Iwashiya_Scripts/tree/main/Photoshop
Meta-URL=https://github.com/Yamonov/Iwashiya_Scripts/tree/main/Illustrator
```

クライアントは、それぞれ次の raw テキストを更新判定の正本として取得します。

```text
https://raw.githubusercontent.com/Yamonov/Iwashiya_Scripts/main/Photoshop/SCRIPTMETA.txt
https://raw.githubusercontent.com/Yamonov/Iwashiya_Scripts/main/Illustrator/SCRIPTMETA.txt
```

`https://github.com/Yamonov/Iwashiya_Scripts/tree/main` のような親ディレクトリ URL は、人が一覧を見るための入口としては使えますが、各ディレクトリ内の `SCRIPTMETA.txt` を自動探索する URL としては扱いません。

## 10. 配布ページタグの互換ルール

配布ページ側は、v1.2 以降では `SCRIPTMETA-DIST-BEGIN` と `SCRIPTMETA-DIST-END` を使います。

v1.1 仕様との互換のため、クライアントは旧形式の `SCRIPTMETA-BEGIN` と `SCRIPTMETA-END` も配布ページ側の候補として扱って構いません。ただし旧形式は非推奨です。

同一ページに複数のメタブロックがある場合、クライアントは次の優先順位で配布ページ用ブロックを選びます。

1. `SCRIPTMETA-DIST-BEGIN` / `SCRIPTMETA-DIST-END` で囲まれたブロック
2. 旧 `SCRIPTMETA-BEGIN` / `SCRIPTMETA-END` のうち、`Meta-URL` を含まないブロック

旧形式ブロックの中で `Meta-URL` を含むものは、script 本体由来のブロックとみなして配布ページ判定から除外します。

## 11. 記述ルール

SCRIPTMETA は次のルールで記述します。

- `Key=Value` 形式
- 区切りは最初の `=` のみ
- UTF-8 テキストを推奨
- 空行は無視
- 不明キーは無視可能
- 複数行の値は禁止
- ただし説明文は `Description-BEGIN` と `Description-END` のブロック形式で記述可能
- HTML の見た目、文字サイズ、色、CSS による表示状態では判定しない
- 取得できた本文テキストだけを対象にする

## 12. キー一覧

| Key | Update Profile script側 | Local Subset | 配布ページ側 | 説明 |
|-----|--------------------------|--------------|--------------|------|
| Script-ID | 必須 | 必須 | 必須 | スクリプト識別子。全配布者の中で一意を強く目指す |
| Version | 必須 | 推奨 | 最新版ページで必須 | バージョン比較に使う |
| Meta-URL | 必須 | 書かない | - | 最初に参照する SCRIPTMETA 情報ページ |
| Latest-URL | - | - | 旧版ページで必須 | 次に参照する SCRIPTMETA 情報ページ |
| Description | 推奨 | 推奨 | 無視 | スクリプト説明文 |
| Target-App | 推奨 | 推奨 | 無視 | 対象アプリ |
| Min-Target-Version | 任意 | - | 無視 | 動作対象バージョン |
| Release-Date | 任意 | 任意 | 無視 | リリース日 |
| Name | 任意 | 任意 | 無視 | 表示名。識別には使わない |
| Author | 推奨。パスワードありで必須 | 任意。パスワードありで必須 | 無視 | 作者名。識別には使わない |
| Edit-Password-SHA256 | 任意 | 任意 | 無視 | 編集UI用のパスワードハッシュ。更新確認には使わない |

## 13. AI向け要点

SCRIPTMETA を扱うプログラムを書く場合は、次を守ってください。

- script 側は `SCRIPTMETA-BEGIN` と `SCRIPTMETA-END` を使います。
- 配布ページ側は `SCRIPTMETA-DIST-BEGIN` と `SCRIPTMETA-DIST-END` を優先します。
- Update Profile の script 側では、`Script-ID`、`Version`、`Meta-URL` を必須とします。
- Local Subset では、`Script-ID` だけを必須とします。
- Local Subset では `Meta-URL` を書かない前提です。
- 更新確認対象にするのは、少なくとも `Version` と `Meta-URL` を持つ script 項目です。
- `Script-ID` は Update Profile でも Local Subset でも必須で、全配布者の中で一意になることを強く目指します。
- `Name` は任意の表示名で、識別には使いません。
- `Author` は作者名で、Update Profile では推奨、Local Subset では任意です。
- `Edit-Password-SHA256` がある場合は `Author` を必須とし、値は `<salt>:<sha256>` 形式で扱います。
- `Edit-Password-SHA256` は編集UI用の確認タグで、更新確認や作者証明には使いません。
- `Description` は `Description-BEGIN` と `Description-END` の間を改行込みで保持します。
- `Description` 内ではキー解析をしてはいけません。
- `Description-END` は、前後の空白を除いた行全体が `Description-END` の場合だけ終端タグです。
- `■ Description-END` は終端タグではありません。
- 配布ページ側の `Description` は無視します。
- `Version` と `Latest-URL` が両方ある配布レコードでは、`Latest-URL` を優先します。
- `Latest-URL` は配布ファイルではなく、次に参照する SCRIPTMETA 情報ページです。
- `Latest-URL` の追跡では循環参照を検出し、追跡回数に上限を設けてください。
- gist、GitHub リポジトリ、GitHub ディレクトリでは、`SCRIPTMETA.txt` を優先してください。
- 自動ダウンロードや自動インストールは行ってはいけません。

AIに指示してパースプログラムを組むときは、「SCRIPTMETA_AI_IMPLEMENTATION_GUIDE.md」を指示書として使用してください。
