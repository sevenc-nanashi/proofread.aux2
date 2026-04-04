# proofread.aux2 タスクリスト

最終更新: 2026-04-04

## 完了

- [x] `register()` 実装
  - [x] `window client` 登録（`proofread.aux2`）
  - [x] 5つのマーカーフィルタを `SubPlugin` として登録
- [x] GUI骨組みの実装（`aviutl2-eframe`）
  - [x] ヘッダー表示
  - [x] `Setup/Run` 画面（設定ボタン、校正開始ボタン、プロンプト入力欄）
  - [x] `Result` 画面（戻る、全体コメント、詳細カード枠）
- [x] マーカープラグイン実装
  - [x] `校正対象（レイヤー、テキスト）@proofread.aux2`（`as_object: true`）
  - [x] `校正対象（単一、テキスト）@proofread.aux2`（`as_object: false`）
  - [x] `校正対象（レイヤー、音声）@proofread.aux2`（`as_object: true`）
  - [x] `校正対象（単一、音声）@proofread.aux2`（`as_object: false`）
  - [x] `校正メモ@proofread.aux2`（`as_object: false`、`内容` テキスト項目あり）
- [x] 設定管理の土台
  - [x] `credentials.json` の保存/読込
  - [x] プリセット定義（Gemini/ChatGPT/Claude/OpenRouter）
- [x] 校正処理の土台
  - [x] プロンプト生成
  - [x] OpenAI互換APIクライアント
  - [x] 結果JSON型 (`all` / `details`) とパース
  - [x] サービス層（テキスト対象のみ実行）
- [x] フォント埋め込み
  - [x] `M+ 1p` を同梱し、GUIの `Proportional` 既定フォントに設定
- [x] GUI「設定」画面の実動作
  - [x] プリセット選択（Gemini/ChatGPT/Claude/OpenRouter）
  - [x] プリセット変更時の `base_url` / `model` 自動反映
  - [x] APIキー発行リンク表示
  - [x] `credentials.json` 保存導線（初回起動時は設定画面を開く）
- [x] プロンプトのプロジェクト保存
  - [x] `on_project_load/on_project_save` で `ProjectFile::deserialize/serialize` を利用
  - [x] `serde` で `project_prompt` をプロジェクト単位で保存・復元
- [x] プロジェクト走査による実マーカー収集
  - [x] `EDIT_HANDLE` 経由でレイヤー/オブジェクト走査
  - [x] 対象ID・レイヤー名・開始時間・色・メモの収集
  - [x] レイヤー対象/単一対象（テキスト・音声）マーカーを判定
- [x] テスト/整形
  - [x] `cargo fmt`
  - [x] `cargo test`（現状 12 tests pass）

## 未着手 / 次段

- [x] 「校正を開始」から `ProofreadService` 実行までのUI接続
- [x] 結果画面への実レスポンス反映（優先度表示の整形含む）
  - [x] `all/details` の基本表示
  - [x] 優先度表示（低/中/高）の整形
- [x] 「ジャンプ」実装（対象位置へカーソル移動）
- [ ] 「メモを追加」実装（対象へ `校正メモ` 付与）
- [x] AIコメント記法の導入（アクションボタン生成）
  - [x] `$[jump ボタン表示名 対象ID]` のパースとボタン表示
  - [x] `$[suggestion 置換後テキスト]`（`\n`エスケープ可）のパースとボタン表示
  - [ ] ボタン押下時の実動作（ジャンプ/置換適用）
    - [x] `jump` の実動作（対象へ移動、存在時focus）
    - [ ] `suggestion` の実動作（置換適用）
- [ ] 音声文字起こし（`whisper-rs`）と音声対象の校正入力対応
- [ ] エラーハンドリング強化（通信失敗/JSON不正時のUI表示）
- [x] プロンプトテンプレート管理の導入（Handlebars）
  - [x] 組み込みデフォルトテンプレートを別ファイル化（`src/prompts/default.hbs`）
  - [x] `DLL` 横 `prompts` フォルダの自動作成（未存在時）
  - [x] `prompts/*.hbs` の読み込みとUIでのテンプレート選択
