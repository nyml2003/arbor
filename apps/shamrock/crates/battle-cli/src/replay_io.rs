use std::error::Error;
use std::fs;
use std::path::Path;

use battle_replay::BattleRecord;

use crate::i18n::Locale;

/**
把 replay 保存到 demo 的默认路径。

现在固定写到 `replays/first-playable-demo.json`，是为了把手动验证流程保持得最短。
*/
pub(crate) fn save_replay(record: &BattleRecord, locale: Locale) -> Result<(), Box<dyn Error>> {
    save_replay_to(Path::new("replays/first-playable-demo.json"), record)?;
    println!(
        "{}",
        locale.replay_saved("replays/first-playable-demo.json")
    );
    Ok(())
}

/**
把 replay 真正写到文件系统。

路径准备和文件写入都收拢在这里，外层只表达“我要保存 replay”。
*/
pub(crate) fn save_replay_to(path: &Path, record: &BattleRecord) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, record.to_pretty_json()?)?;
    Ok(())
}
