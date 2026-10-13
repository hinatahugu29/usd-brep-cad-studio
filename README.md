# USD-BRep CAD Studio

NVIDIA Omniverse の **USD-BRep (SMLib カーネル)** をバックエンドに採用した、Rust 製のプロフェッショナル 3D CAD アプリケーションです。UI は Fusion 360 やオープンソース CAD [SolidCraft](https://github.com/bherbruck/solidcraft)(`Reference/solidcraft` に参考収録)のワークフローを参考にしていますが、名称・テーマは独自のものです。

---

## 🌟 主な機能と GUI 構成

### 1. **Fusion 360 スタイルの CAD GUI & ダイレクト操作**
- **⚙ Fusion 360 風 ユーザーパラメータ一覧テーブル ([`src/params.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/params.rs))**:
  - ツールバーの `[fx Parameters]` または S キーから `params` でアクセス。
  - Fusion 360 の「Change Parameters」ダイアログを忠実に再現した変数管理テーブル（Name, Unit, Value, Comment）。
  - パラメータ値（`base_width`, `base_length`, `base_height`, `hole_radius` など）をドラッグまたは数値変更すると、USD-BRep カーネルが連動してソリッドを瞬時に自動再構築（Rebuild）。
- **📊 リアルタイム 3D 断面解析 (Section Analysis / Slicing) ([`src/section_analysis.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/section_analysis.rs))**:
  - ツールバーの `[📊 Section]` または S キーから `section` でトグル。
  - Fusion 360 の「Section Analysis（断面解析）」と同様に、切断軸（X, Y, Z 平面）、オフセットスライサー（mm）、反転（Flip Direction）で内部構造を自由に検査。
  - GPU シェーダーレベルでの高速クリッピングにより、リアルタイムに空洞や内部フィーチャーを確認可能。
- **⌨ S キー クイックコマンドパレット ([`src/shortcut_box.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/shortcut_box.rs))**:
  - Fusion 360 や SolidCraft で最も多用される **S キー検索ボックス** を完全再現。
  - キーボードの `S` キーを押すとマウスポインタ位置にスマート検索ウィンドウが瞬時にポップアップ。
  - `box`, `cyl`, `sketch`, `fillet`, `chamfer`, `measure`, `params`, `section`, `fit`, `step` などを入力して素早くコマンドを実行可能（矢印キー・Enter・Esc に対応）。
- **📏 インタラクティブ 3D 寸法アノテーション ([`src/measure.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/measure.rs))**:
  - ツールバーの `[📏 Measure]` または S キーからオン/オフ切り替え。
  - 選択中のソリッド（または全パーツ）に対して、CAD 図面のような **三次元寸法引出線（Extension Lines）と寸法バッジ（幅 X, 奥行 Y, 高さ Z mm）** を 3D ビューポート上にリアルタイム投影オーバーレイ表示。
- **🖱 右クリック コンテキストメニュー**:
  - ビューポート上でパーツを右クリックすると、手元で即座に操作できるクイックメニューを展開:
    - 📋 **Duplicate**（ソリッドの複製）
    - 🧊 **Apply Fillet** / 📐 **Apply Chamfer**
    - 🗑 **Delete**（削除）
    - 📤 **Export STEP**
- **🎨 材質・カラーアピアランス選択 ([`src/dialogs.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/dialogs.rs))**:
  - ソリッドごとに外観材質（Steel Blue, Anodized Silver, Polished Brass, Crimson Red, Machine Green, Carbon Dark）をワンクリックで割り当て、3D レンダラーにリアルタイム反映。
- **3D ダイレクト操作ギズモ ([`src/gizmo.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/gizmo.rs))**:
  - 選択中の 3D ソリッド上に **X(赤), Y(緑), Z(青) の 3 軸移動ギズモ（Triad）** を画面上に直接描画。
  - マウスで矢印ハンドルを掴んでドラッグすることで、3D ビューポート内でソリッドの位置を直感的にダイレクト操作（0.5mm グリッドスナップ対応）。
- **インタラクティブ 2D スケッチシステム ([`src/sketch.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/sketch.rs))**:
  - `[✏ Create Sketch]` ボタンで 2D スケッチモードへ移行。
  - 基準平面（XY 平面: 上面、XZ 平面: 正面、YZ 平面: 右側面）での高精度スケッチ。
  - レイキャストによる正確な 3D 平面座標取得とリアルタイム座標表示。
  - ツール:
    - 📏 **Line**: 直線の直接描画
    - ⬛ **Rectangle**: 長方形プロファイルの直接描画
    - ⭕ **Circle**: 円プロファイルの直接描画
  - **3D ソリッド自動押し出し (Extrude)**: 描画したスケッチプロファイルから直接 3D ソリッド（Box や Cylinder 等）を生成してシーンに統合。
- **スケッチパレット ([`src/sketch_palette.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/sketch_palette.rs))**:
  - スケッチ編集中にビューポート右側に表示される専用コントロール。
  - スケッチ平面切り替え・`📐 Look At Sketch`（スケッチ平面へのカメラ自動正対）。
  - 押し出し深さ（Extrude Depth mm）スライダー。
  - `✓ Finish & Extrude`（スケッチ完了・3D化）および `✕ Cancel`。
- **Nord-inspired CAD ダークテーマ ([`src/theme.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/theme.rs))**:
  - `#2e3440` (Nord Slate) による疲れにくく上質な 3D ビューポート背景。
  - CAD に最適化されたカラーパレット、ボーダー、アクセントカラー。
- **リボンワークスペースツールバー ([`src/toolbar.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/toolbar.rs))**:
  - ワークスペースタブ（`SOLID`, `SURFACE`, `MESH`, `SHEET METAL`, `UTILITIES`）。
  - カテゴリ別コマンドリボン（**SKETCH & CREATE**, **MODIFY**, **VIEW**, **EXPORT**）。
- **モデルブラウザツリー ([`src/browser.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/browser.rs))**:
  - `Document Settings`（単位 mm、SMLib カーネル）。
  - `Named Views`（Isometric, Top, Front, Right）。
  - `Origin`（原点、座標軸、基準平面の可視性切り替え）。
  - `Bodies`（ソリッドボディ、材質）。
  - `Features Tree`（各フィーチャーの表示・非表示、選択、削除）。
  - `Mass Properties`（体積 mm³、表面積 mm²、BBox 等のリアルタイム幾何解析）。
- **パラメトリックタイムライン ([`src/timeline.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/timeline.rs))**:
  - Fusion 360 / SolidCraft 風の画面下部履歴タイムライン。
  - トランスポートコントロール（⏮ 最初へ, ◀ 1ステップ戻る, ▶ 1ステップ進む, ⏭ 最新へ）。
  - フィーチャーカード（アイコン、名前、ブーリアンタイプタグ）。
  - **ロールバック機能**: タイムラインバーを巻き戻すと、その時点までのソリッド形状がリアルタイムに再評価・3Dレンダリング。
- **3D ViewCube & 座標軸ギズモ ([`src/viewcube.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/viewcube.rs))**:
  - ビューポート右上のインタラクティブ ViewCube（TOP, FRONT, RIGHT, ISO）クリックで即座に視点切り替え。
  - ビューポート左下の 3D 座標軸（X: 赤, Y: 緑, Z: 青）インジケーター。
- **ビューポート HUD ナビゲーションバー**:
  - ビューポート下部のクイックアクセスチップ（Fit to View, ISO, Top, Front）。
- **フィーチャープロパティダイアログ ([`src/dialogs.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/dialogs.rs))**:
  - 選択したソリッドの寸法パラメータ、位置オフセット、回転、ブーリアン演算モード（Base, Join, Cut, Intersect）、エッジ処理（Fillet, Chamfer）、外観マテリアル。

---

### 2. **USD-BRep (SMLib) パラメトリックカーネル & 3D レンダラー**
- **正確な B-Rep モデリング**:
  - 基本形状（Box, Cylinder, Sphere, Cone, Torus）の自由な配置と寸法調整
  - ブーリアン演算（**Join [+]**, **Cut [-]**, **Intersect [∩]**）
  - エッジ処理（フィレット / 面取り）
- **高速 3D ビューポート (OpenGL / Glow)**:
  - 左ドラッグ: オービット回転（非ギズモ時）
  - 右 / 中ドラッグ: パン（平行移動）
  - スクロール: ズーム
  - `F` キー: モデル全体へ自動フィット (Fit to View)
  - クッキリとした CAD フィーチャーライン（稜線）と 3D グリッド
- **CAD & 3D エクスポート**:
  - **STEP 出力**: 製造業標準の正確な AP242 B-Rep 幾何情報を保持した `.step`
  - **GLB 出力**: Web / 3D ビューア向けの `.glb`
  - **STL 出力**: 3D プリンタ向けの `.stl`

---

## 📁 ディレクトリ構成

- [`src/main.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/main.rs): アプリケーションのエントリポイント
- [`src/app.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/app.rs): アプリケーション状態管理とメインループ
- [`src/shortcut_box.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/shortcut_box.rs): S キー クイックコマンド検索パレット
- [`src/measure.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/measure.rs): 3D 寸法測定・引出線アノテーション描画
- [`src/gizmo.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/gizmo.rs): 3D 移動ギズモ（Triad）とダイレクト操作
- [`src/sketch.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/sketch.rs): 2D スケッチ描画および押し出し（Extrude）
- [`src/sketch_palette.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/sketch_palette.rs): スケッチパレット（平面選択、Look At、完了ボタン）
- [`src/theme.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/theme.rs): Nord ダークテーマ（Tokens、スタイル）
- [`src/toolbar.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/toolbar.rs): ワークスペースタブ・リボンツールバー
- [`src/browser.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/browser.rs): モデル階層ブラウザ・原点・質量特性パネル
- [`src/timeline.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/timeline.rs): 履歴タイムライン・ロールバック再生コントロール
- [`src/viewcube.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/viewcube.rs): 3D ViewCube & 座標軸ギズモ
- [`src/dialogs.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/dialogs.rs): フィーチャーパラメータ編集ダイアログ・マテリアル選択
- [`src/params.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/params.rs): Fusion 360 風ユーザーパラメータ管理テーブル
- [`src/section_analysis.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/section_analysis.rs): リアルタイム 3D 断面解析（スライスコントロール）
- [`src/renderer.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/renderer.rs): OpenGL シェーダー（断面クリッピング対応）、メッシュ・稜線・グリッドの 3D 描画
- [`src/camera.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/camera.rs): オービットカメラ制御
- [`src/kernel.rs`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/src/kernel.rs): NVIDIA USD-BRep / SMLib カーネル連携
- [`Reference/solidcraft`](file:///c:/Users/T03000/Desktop/CODE/NVIDIA-CAD-TEST/Reference/solidcraft): リファレンス用 SolidCraft ソースコード

---

## 🚀 起動方法

```bash
cargo run
```
