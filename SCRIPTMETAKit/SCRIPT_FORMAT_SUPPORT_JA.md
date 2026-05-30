# script format support matrix

この文書は、SCRIPTMETAKit が現在標準対応している script file と、将来 option として対応可能な file format を整理するものです。

重要な前提:

- kit は profile や Adobe app 分類を持たない。app 側が root/profile/app 実行方針を管理する。
- kit は file list、runtime hint、SCRIPTMETA metadata、読み出し diagnostics を返す。
- 「Adobe アプリへ実行できるか」は、kit の scan 対象に含めるかとは別の判定とする。
- 標準対応は汎用 script utility として安全な範囲に限定し、host app が必要なら `ExtensionPolicy::new(...)` で追加する。

## 現在の標準対応

`ExtensionPolicy::script_default()` の現行値です。

| extension | runtime hint | Adobe アプリへ実行可能か | format | comment | shebang / 判別 header | SCRIPTMETA 埋め込み | SCRIPTMETA 読み出し |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `.js` | shebang が JXA なら `JavaScriptForAutomation`、それ以外は `AdobeJavaScript` | 可。Adobe ExtendScript として実行可能な app あり。macOS では JXA として Apple Events 制御も可能 | text | `/* ... */`, `//` | 先頭 bytes が `#!/usr/bin/osascript -l JavaScript` なら JXA と判定。通常 Adobe JS には確実な header なし | 可 | 速い。prefix text scan で十分。ただし Adobe JS と WSH JScript の区別は extension だけでは不可 |
| `.jsx` | `AdobeJavaScript` | 可。ExtendScript 対応 app で実行可能 | text | `/* ... */`, `//` | なし。`#target` など Adobe directive は runtime hint には使えるが標準判定にはまだ使わない | 可 | 速い。prefix text scan |
| `.jsxinc` | `AdobeJavaScript` | 単体実行は原則非推奨。`.jsx` から include する用途。wrapper があれば実行可能 | text | `/* ... */`, `//` | なし | 可 | 速い。prefix text scan |
| `.jsxbin` | `AdobeJavaScript` | 可。ExtendScript host が JSXBIN を受け付ける場合 | obfuscated / encoded | 実質不可 | `@JSXBIN` prefix | 原則不可。コメント保持を前提にできない | 難。現状は file として拾えるだけ。内蔵 SCRIPTMETA は信頼しない |
| `.scpt` | `AppleScript` | macOS で可。OSA/`osascript` から Adobe app を Apple Events で制御可能 | compiled / binary | source 側は `(* ... *)`, `--`, `#` | OSA compiled script。shebang なし | 可能だが compiled 後に `osadecompile` または OSA API が必要 | 中。process 起動の `osadecompile` は遅くなり得る。cache 必須 |
| `.applescript` | shebang が JXA なら `JavaScriptForAutomation`、それ以外は `AppleScript` | macOS で可。Apple Events で Adobe app を制御可能 | text | `(* ... *)`, `--`, `#` | 先頭 bytes が `#!/usr/bin/osascript -l JavaScript` なら JXA | 可 | 速い。prefix text scan |
| `.jxa` | `JavaScriptForAutomation` | macOS で可。`osascript -l JavaScript` / OSA から Adobe app を Apple Events で制御可能 | text | `/* ... */`, `//` | shebang 任意。extension で JXA と扱う | 可 | 速い。prefix text scan |
| `.idjs` | `AdobeUxp` | InDesign UXP script。外部 app からの直接実行は ExtendScript/AppleScript/COM ほど単純ではない | text | `/* ... */`, `//` | UXP script extension | 可 | 速い。prefix text scan |
| `.psjs` | `AdobeUxp` | Photoshop UXP script。外部 app からの直接実行は ExtendScript/AppleScript/COM ほど単純ではない | text | `/* ... */`, `//` | UXP script extension | 可 | 速い。prefix text scan |

## 将来 option として対応可能な候補

標準には入れず、host app が必要な場合に追加する候補です。

| extension / container | 想定 runtime | Adobe アプリへ実行可能か | format | comment | shebang / 判別 header | SCRIPTMETA 埋め込み | SCRIPTMETA 読み出し |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `.vbs` | VBScript / Windows Script Host | Windows で可。COM/OLE Automation で Illustrator/Photoshop などを制御可能 | text | `'`, `Rem`。block comment なし | なし | 可。line comment で埋め込む | 速い。ただし parser は `'` と `Rem` の comment prefix 対応が必要 |
| `.js` as WSH JScript | JScript / Windows Script Host | Windows で可。COM/OLE Automation を使う | text | `/* ... */`, `//` | なし。Adobe ExtendScript の `.js` と衝突する | 可 | 速い。ただし Adobe JS との区別には app 側 policy、header、root profile のいずれかが必要 |
| `.wsf` | Windows Script File | Windows で可。JScript/VBScript job から COM 制御 | XML text | XML は `<!-- ... -->`。内部 script は JS/VBS の comment | XML declaration、`<job>` / `<script language=...>` | 可 | 中。XML parse か XML comment 除去が必要。prefix scan だけでも可能だが堅牢性は低い |
| `.ps1` | PowerShell | Windows で可。COM Automation を使う | text | `<# ... #>`, `#` | `#!/usr/bin/env pwsh` は cross-platform 用に存在し得る。Windows では主判定に使わない | 可 | 速い。現行 parser は `#` を処理できる。block comment の扱いは追加すると良い |
| `.psm1` | PowerShell module | 単体実行ではなく module。launcher が呼べば Adobe 制御可能 | text | `<# ... #>`, `#` | module extension | 可 | 速い。`.ps1` と同等 |
| `.bas`, `.cls`, `.frm` | exported VBA module/class/form | 直接 Adobe app に渡すものではない。Office など VBA host から COM で Adobe app を制御 | text | `'`, `Rem`。block comment なし | `Attribute VB_Name = ...` など VBA module attribute | 可 | 速い。parser は `'` と `Rem` 対応が必要 |
| `.xlsm`, `.xlam`, `.docm`, `.dotm`, `.pptm`, `.ppam` | Office VBA macro container | 直接 Adobe app に渡すものではない。Office macro から COM で制御 | ZIP package + `vbaProject.bin` | module source は `'`, `Rem` | Open XML package、VBA project streams | module 内なら可 | 難。container 展開、VBA project 取り出し、module text 抽出が必要。cache 必須 |
| `.xlsb`, `.xls`, `.doc`, `.ppt` | Office VBA legacy/binary container | 同上 | binary container | module source は `'`, `Rem` | OLE compound / binary format | module 内なら可 | 難。binary container 解析が必要。password/encryption で読めない場合あり |
| `.accdb`, `.accde` | Access VBA / compiled Access app | Access host から COM 制御可能 | database / compiled container | module source は `'`, `Rem` | Access database container | `.accdb` は可能、`.accde` は困難 | 難。Access 専用抽出が必要。`.accde` は source が失われるため基本不可 |
| `.hta` | HTML Application | Windows で可。JScript/VBScript から COM 制御 | HTML text | HTML `<!-- ... -->`、内部 JS/VBS comment | HTML / HTA tags | 可 | 中。HTML parse または script block 抽出が必要 |
| `.jse`, `.vbe` | encoded JScript/VBScript | Windows で実行可能だが古い encoded script | encoded text | source comment は失われる前提 | encoded header | 原則不可 | 難または不可。metadata は sidecar 推奨 |
| `.sh` | POSIX shell | Adobe 直接ではない。macOS で `osascript` や Adobe CLI wrapper を呼べる | text | `#` | shebang あり | 可 | 速い。ただし標準対象からは外す |
| `.command` | macOS Terminal command | Adobe 直接ではない。shell と同じ | text | `#` | shebang 任意 | 可 | 速い。ただし標準対象からは外す |
| `.bat`, `.cmd` | Windows command script | Adobe 直接ではない。`cscript`, `powershell`, app launcher を呼べる | text | `Rem`, `::` | なし | 可 | 速い。ただし script metadata 対象としては優先度低 |

## package / binary は別扱い

以下は script file list に混ぜず、将来必要なら package metadata adapter として扱う方が安全です。

| extension / container | 扱い |
| --- | --- |
| `.ccx`, `.zxp` | Adobe UXP/CEP 配布 package。ZIP container として manifest や内部 script を読むことは可能だが、単一 script file ではない |
| UXP plugin folder | `manifest.json` と JS/HTML/CSS の project。file list ではなく package/project として扱う |
| CEP extension folder | HTML/JS/CSS + ExtendScript の project。内部 `.jsx` は読めるが package 単位の扱いが必要 |
| `.aip`, `.plugin`, `.8bf`, `.dll`, `.exe` | binary plugin / executable。SCRIPTMETA を直接読む対象にしない。必要なら resource または sidecar metadata |
| `.txt` | script として実行できないため標準から外す。説明文や sidecar として扱う場合だけ別 policy |

## 読み出し難易度の基準

| difficulty | 内容 |
| --- | --- |
| 速い | text file の先頭 `max_prefix_bytes` だけを読む。現在の scan/cache 設計で処理できる |
| 中 | XML/HTML parse、または `.scpt` decompile のように追加処理が必要。cache がないと folder scan で体感が悪くなりやすい |
| 難 | binary/encoded/container 解析が必要。抽出失敗理由を diagnostics として返す必要がある |
| 不可 | comment/source が保持されない、または runtime package から metadata を安定して復元できない。sidecar metadata を優先する |

## 実装上の方針

- extension policy は app から指定できるようにする。標準対応に Windows/Office/container 系を混ぜない。
- runtime hint は extension と軽い header/shebang だけで返す。実際の実行方法は app 側が決める。
- text script は prefix scan を基本にする。巨大 file の全読み込みは避ける。
- `.scpt`、Office macro container、Adobe package は platform adapter / package adapter が text と diagnostics を返す。
- VBA/VBScript/Batch/XML 系に対応する前に、parser の `clean_line` を拡張する。
  - `'`
  - `Rem`
  - `::`
  - `<!--`
  - `-->`
  - `<#`
  - `#>`
- binary/encoded/container で SCRIPTMETA を読めない場合は、失敗を無音で潰さず diagnostics に残す。

## 参考

- Adobe Illustrator scripting language support: <https://ai-scripting.docsforadobe.dev/introduction/scriptingLanguageSupport/>
- Adobe Photoshop scripting: <https://helpx.adobe.com/photoshop/using/scripting.html>
- Adobe InDesign UXP scripts and plugins: <https://developer.adobe.com/indesign/uxp/introduction/next-steps/script-and-plugin/>
- Adobe Photoshop UXP scripting: <https://developer.adobe.com/photoshop/uxp/scripting/>
- AppleScript lexical conventions: <https://developer.apple.com/library/archive/documentation/AppleScript/Conceptual/AppleScriptLangGuide/conceptual/ASLR_lexical_conventions.html>
- JavaScript lexical grammar: <https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Lexical_grammar>
- VBA `Rem` statement: <https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/rem-statement>
- Office VBA file format: <https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-ovba/b39ac32f-0ce1-4533-9297-2ff3ff62c9ec>
- PowerShell comments: <https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_comments>
