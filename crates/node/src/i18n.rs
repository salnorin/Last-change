//! Menu translations. Each row: key, then text in the order of [`LANGS`].

/// (code, native name) in table column order.
pub const LANGS: [(&str, &str); 9] = [
    ("en", "English"),
    ("de", "Deutsch"),
    ("es", "Español"),
    ("ar", "العربية"),
    ("fr", "Français"),
    ("zh", "中文"),
    ("ru", "Русский"),
    ("tr", "Türkçe"),
    ("fa", "فارسی"),
];

type Row = (&'static str, [&'static str; 9]);

#[rustfmt::skip]
pub const T: &[Row] = &[
    ("title", ["Last-change wallet", "Last-change Wallet", "Monedero Last-change", "محفظة Last-change", "Portefeuille Last-change", "Last-change 钱包", "Кошелёк Last-change", "Last-change cüzdanı", "کیف پول Last-change"]),
    ("m1", ["1) New wallet", "1) Neue Wallet", "1) Nuevo monedero", "1) محفظة جديدة", "1) Nouveau portefeuille", "1) 新建钱包", "1) Новый кошелёк", "1) Yeni cüzdan", "۱) کیف پول جدید"]),
    ("m2", ["2) Restore from words", "2) Aus Wörtern wiederherstellen", "2) Restaurar con palabras", "2) استعادة من الكلمات", "2) Restaurer depuis les mots", "2) 用助记词恢复", "2) Восстановить по словам", "2) Kelimelerden geri yükle", "۲) بازیابی با کلمات"]),
    ("m3", ["3) Show wallet info", "3) Wallet-Infos anzeigen", "3) Ver información del monedero", "3) عرض معلومات المحفظة", "3) Afficher le portefeuille", "3) 查看钱包信息", "3) Показать кошелёк", "3) Cüzdan bilgisini göster", "۳) نمایش اطلاعات کیف پول"]),
    ("m4", ["4) Exit", "4) Beenden", "4) Salir", "4) خروج", "4) Quitter", "4) 退出", "4) Выход", "4) Çıkış", "۴) خروج"]),
    ("m5", ["5) Paper trade journal / P&L", "5) Papier-Handelsjournal / G&V", "5) Diario de operaciones en papel / P&G", "5) سجل التداول الورقي / الربح والخسارة", "5) Journal de trading fictif / P&L", "5) 模拟交易日志 / 盈亏", "5) Журнал бумажных сделок / P&L", "5) Kâğıt işlem günlüğü / K&Z", "۵) ثبت معامله کاغذی / دیدن سود و ضرر"]),
    ("choose", ["Choose (1-5): ", "Auswahl (1-5): ", "Elija (1-5): ", "اختر (1-5): ", "Choisissez (1-5) : ", "请选择 (1-5): ", "Выберите (1-5): ", "Seçin (1-5): ", "انتخاب کنید (۱ تا ۵): "]),
    ("pj_title", ["PAPER journal (local only, not on the network, not real money)", "PAPIER-Journal (nur lokal, nicht im Netzwerk, kein echtes Geld)", "Diario en PAPEL (solo local, no en la red, no es dinero real)", "سجل ورقي (محلي فقط، ليس على الشبكة، ليس مالاً حقيقياً)", "Journal FICTIF (local uniquement, pas sur le réseau, pas d'argent réel)", "模拟日志（仅本地，不上网络，非真实资金）", "БУМАЖНЫЙ журнал (только локально, не в сети, не реальные деньги)", "KÂĞIT günlük (yalnızca yerel, ağda değil, gerçek para değil)", "دفتر معاملات کاغذی (فقط محلی، روی شبکه نمی‌رود، پول واقعی نیست)"]),
    ("pj_menu", ["  1) Buy  2) Sell  3) Positions & P&L  4) List trades  5) Export CSV  6) Back", "  1) Kaufen  2) Verkaufen  3) Positionen & G&V  4) Trades auflisten  5) CSV-Export  6) Zurück", "  1) Comprar  2) Vender  3) Posiciones y P&G  4) Listar operaciones  5) Exportar CSV  6) Volver", "  1) شراء  2) بيع  3) المراكز والربح  4) قائمة الصفقات  5) تصدير CSV  6) رجوع", "  1) Acheter  2) Vendre  3) Positions & P&L  4) Lister les trades  5) Export CSV  6) Retour", "  1) 买入  2) 卖出  3) 持仓与盈亏  4) 交易列表  5) 导出 CSV  6) 返回", "  1) Купить  2) Продать  3) Позиции и P&L  4) Список сделок  5) Экспорт CSV  6) Назад", "  1) Al  2) Sat  3) Pozisyonlar & K&Z  4) İşlemleri listele  5) CSV dışa aktar  6) Geri", "  ۱) خرید  ۲) فروش  ۳) پوزیشن‌ها و سود و ضرر  ۴) فهرست معاملات  ۵) خروجی CSV  ۶) بازگشت"]),
    ("pj_choose", ["Choose (1-6): ", "Auswahl (1-6): ", "Elija (1-6): ", "اختر (1-6): ", "Choisissez (1-6) : ", "请选择 (1-6): ", "Выберите (1-6): ", "Seçin (1-6): ", "انتخاب کنید (۱ تا ۶): "]),
    ("pj_asset", ["Asset (e.g. PAXG): ", "Asset (z. B. PAXG): ", "Activo (p. ej. PAXG): ", "الأصل (مثلاً PAXG): ", "Actif (ex. PAXG) : ", "资产（如 PAXG）: ", "Актив (напр. PAXG): ", "Varlık (örn. PAXG): ", "دارایی (مثلاً PAXG): "]),
    ("pj_qty", ["Quantity: ", "Menge: ", "Cantidad: ", "الكمية: ", "Quantité : ", "数量: ", "Количество: ", "Miktar: ", "مقدار: "]),
    ("pj_price", ["Price: ", "Preis: ", "Precio: ", "السعر: ", "Prix : ", "价格: ", "Цена: ", "Fiyat: ", "قیمت: "]),
    ("pj_fee", ["Fee (optional, Enter = 0): ", "Gebühr (optional, Enter = 0): ", "Comisión (opcional, Enter = 0): ", "الرسوم (اختياري، Enter = 0): ", "Frais (optionnel, Entrée = 0) : ", "手续费（可选，回车 = 0）: ", "Комиссия (необязательно, Enter = 0): ", "Ücret (isteğe bağlı, Enter = 0): ", "کارمزد (اختیاری، Enter = صفر): "]),
    ("pj_note", ["Note (optional): ", "Notiz (optional): ", "Nota (opcional): ", "ملاحظة (اختياري): ", "Note (optionnel) : ", "备注（可选）: ", "Заметка (необязательно): ", "Not (isteğe bağlı): ", "یادداشت (اختیاری): "]),
    ("pj_cur", ["Current price for unrealized P&L (Enter to skip): ", "Aktueller Preis für unrealisierte G&V (Enter = überspringen): ", "Precio actual para P&G no realizada (Enter para omitir): ", "السعر الحالي للربح غير المحقق (Enter للتخطي): ", "Prix actuel pour le P&L latent (Entrée pour passer) : ", "当前价格，用于计算浮动盈亏（回车跳过）: ", "Текущая цена для нереализованного P&L (Enter — пропустить): ", "Gerçekleşmemiş K&Z için güncel fiyat (atlamak için Enter): ", "قیمت فعلی برای سود و ضرر باز (Enter برای رد کردن): "]),
    ("pj_saved", ["Saved (paper).", "Gespeichert (Papier).", "Guardado (papel).", "تم الحفظ (ورقي).", "Enregistré (fictif).", "已保存（模拟）。", "Сохранено (бумажно).", "Kaydedildi (kâğıt).", "ذخیره شد (کاغذی)."]),
    ("pj_csv", ["CSV written to", "CSV gespeichert unter", "CSV guardado en", "تم حفظ CSV في", "CSV écrit dans", "CSV 已写入", "CSV записан в", "CSV şuraya yazıldı:", "فایل CSV ذخیره شد در"]),
    ("words_q", ["How many words? 12 / 15 / 18 / 24 (24 recommended): ", "Wie viele Wörter? 12 / 15 / 18 / 24 (24 empfohlen): ", "¿Cuántas palabras? 12 / 15 / 18 / 24 (se recomienda 24): ", "كم عدد الكلمات؟ 12 / 15 / 18 / 24 (يُنصح بـ 24): ", "Combien de mots ? 12 / 15 / 18 / 24 (24 recommandé) : ", "多少个词？12 / 15 / 18 / 24（推荐 24）: ", "Сколько слов? 12 / 15 / 18 / 24 (рекомендуется 24): ", "Kaç kelime? 12 / 15 / 18 / 24 (24 önerilir): ", "چند کلمه؟ ۱۲ / ۱۵ / ۱۸ / ۲۴ (۲۴ توصیه می‌شود): "]),
    ("bad_words", ["Invalid length. Use 12, 15, 18 or 24.", "Ungültige Länge. Nur 12, 15, 18 oder 24.", "Longitud no válida. Use 12, 15, 18 o 24.", "طول غير صالح. استخدم 12 أو 15 أو 18 أو 24.", "Longueur invalide. Utilisez 12, 15, 18 ou 24.", "长度无效。请用 12、15、18 或 24。", "Неверная длина. Используйте 12, 15, 18 или 24.", "Geçersiz uzunluk. 12, 15, 18 veya 24 kullanın.", "طول نامعتبر است. ۱۲، ۱۵، ۱۸ یا ۲۴ را وارد کنید."]),
    ("enter_words", ["Enter your words separated by spaces: ", "Wörter mit Leerzeichen getrennt eingeben: ", "Escriba sus palabras separadas por espacios: ", "أدخل كلماتك مفصولة بمسافات: ", "Entrez vos mots séparés par des espaces : ", "请输入助记词，用空格分隔: ", "Введите слова через пробел: ", "Kelimelerinizi boşlukla ayırarak girin: ", "کلمات خود را با فاصله وارد کنید: "]),
    ("no_wallet", ["No wallet yet. Choose 1 or 2.", "Noch keine Wallet. Wähle 1 oder 2.", "Aún no hay monedero. Elija 1 o 2.", "لا توجد محفظة بعد. اختر 1 أو 2.", "Pas encore de portefeuille. Choisissez 1 ou 2.", "还没有钱包。请选 1 或 2。", "Кошелька ещё нет. Выберите 1 или 2.", "Henüz cüzdan yok. 1 veya 2'yi seçin.", "هنوز کیف پولی نیست. ۱ یا ۲ را انتخاب کنید."]),
    ("keys", ["Key types: Ed25519 + ML-DSA-65 (FIPS 204) hybrid signatures; X25519 + ML-KEM-768 (FIPS 203) encryption. All derived from your words.", "Schlüssel: Hybrid-Signaturen Ed25519 + ML-DSA-65 (FIPS 204); Verschlüsselung X25519 + ML-KEM-768 (FIPS 203). Alles aus deinen Wörtern abgeleitet.", "Claves: firmas híbridas Ed25519 + ML-DSA-65 (FIPS 204); cifrado X25519 + ML-KEM-768 (FIPS 203). Todo derivado de sus palabras.", "المفاتيح: توقيعات هجينة Ed25519 + ML-DSA-65 (FIPS 204)؛ تشفير X25519 + ML-KEM-768 (FIPS 203). كلها مشتقة من كلماتك.", "Clés : signatures hybrides Ed25519 + ML-DSA-65 (FIPS 204) ; chiffrement X25519 + ML-KEM-768 (FIPS 203). Tout est dérivé de vos mots.", "密钥：Ed25519 + ML-DSA-65 (FIPS 204) 混合签名；X25519 + ML-KEM-768 (FIPS 203) 加密。全部由你的助记词派生。", "Ключи: гибридные подписи Ed25519 + ML-DSA-65 (FIPS 204); шифрование X25519 + ML-KEM-768 (FIPS 203). Всё выводится из ваших слов.", "Anahtarlar: Ed25519 + ML-DSA-65 (FIPS 204) hibrit imza; X25519 + ML-KEM-768 (FIPS 203) şifreleme. Hepsi kelimelerinizden türetilir.", "نوع کلیدها: امضای ترکیبی Ed25519 + ML-DSA-65 (FIPS 204)؛ رمزنگاری X25519 + ML-KEM-768 (FIPS 203). همه از کلمات شما ساخته می‌شوند."]),
    ("bye", ["Bye.", "Tschüss.", "Adiós.", "مع السلامة.", "Au revoir.", "再见。", "До свидания.", "Hoşça kal.", "خدانگهدار."]),
    ("unknown", ["Unknown choice.", "Unbekannte Auswahl.", "Opción desconocida.", "خيار غير معروف.", "Choix inconnu.", "无效选项。", "Неизвестный выбор.", "Bilinmeyen seçim.", "گزینه‌ی نامعتبر."]),
    ("top1", ["1) Wallet", "1) Wallet", "1) Monedero", "1) المحفظة", "1) Portefeuille", "1) 钱包", "1) Кошелёк", "1) Cüzdan", "۱) کیف پول"]),
    ("top2", ["2) Account view (own / other address)", "2) Kontoansicht (eigene / andere Adresse)", "2) Vista de cuenta (propia / otra dirección)", "2) عرض الحساب (عنواني / عنوان آخر)", "2) Vue du compte (propre / autre adresse)", "2) 账户查看（自己 / 其他地址）", "2) Просмотр счёта (свой / другой адрес)", "2) Hesap görünümü (kendi / başka adres)", "۲) مشاهده‌ی حساب (خودم / آدرس دیگر)"]),
    ("top3", ["3) Explorer", "3) Explorer", "3) Explorador", "3) المستكشف", "3) Explorateur", "3) 浏览器", "3) Обозреватель", "3) Gezgin", "۳) کاوشگر"]),
    ("top4", ["4) Send (info)", "4) Senden (Info)", "4) Enviar (info)", "4) إرسال (معلومات)", "4) Envoyer (info)", "4) 发送（说明）", "4) Отправить (инфо)", "4) Gönder (bilgi)", "۴) ارسال (اطلاعات)"]),
    ("top5", ["5) Exit", "5) Beenden", "5) Salir", "5) خروج", "5) Quitter", "5) 退出", "5) Выход", "5) Çıkış", "۵) خروج"]),
    ("wm4", ["4) Paper trade journal / P&L", "4) Papier-Handelsjournal / G&V", "4) Diario de operaciones en papel / P&G", "4) سجل التداول الورقي / الربح والخسارة", "4) Journal de trading fictif / P&L", "4) 模拟交易日志 / 盈亏", "4) Журнал бумажных сделок / P&L", "4) Kâğıt işlem günlüğü / K&Z", "۴) ثبت معامله کاغذی / دیدن سود و ضرر"]),
    ("wm5", ["5) Back", "5) Zurück", "5) Volver", "5) رجوع", "5) Retour", "5) 返回", "5) Назад", "5) Geri", "۵) بازگشت"]),
    ("warn", ["!!! WARNING: if you lose these words, this wallet is unrecoverable FOREVER. Never share your words or private key with anyone.", "!!! WARNUNG: Wer diese Wörter verliert, kann die Wallet NIE wiederherstellen. Wörter und privaten Schlüssel niemals weitergeben.", "!!! AVISO: si pierde estas palabras, el monedero será irrecuperable PARA SIEMPRE. Nunca comparta sus palabras ni su clave privada.", "!!! تحذير: إذا فقدت هذه الكلمات فلن تستعيد المحفظة أبداً. لا تشارك كلماتك أو مفتاحك الخاص مع أي أحد.", "!!! ATTENTION : si vous perdez ces mots, le portefeuille est irrécupérable POUR TOUJOURS. Ne partagez jamais vos mots ni votre clé privée.", "!!! 警告：丢失这些助记词，钱包将永远无法恢复。切勿向任何人透露助记词或私钥。", "!!! ВНИМАНИЕ: потеряв эти слова, вы НАВСЕГДА потеряете кошелёк. Никому не сообщайте слова и закрытый ключ.", "!!! UYARI: bu kelimeleri kaybederseniz cüzdan SONSUZA DEK kurtarılamaz. Kelimelerinizi ve özel anahtarınızı asla paylaşmayın.", "!!! هشدار: اگر این کلمات را گم کنید، کیف پول برای همیشه غیرقابل بازیابی است. کلمات و کلید خصوصی را هرگز به کسی ندهید."]),
    ("key_q", ["Show private key (master secret) now? (y/N): ", "Privaten Schlüssel (Master-Secret) jetzt anzeigen? (j/N): ", "¿Mostrar la clave privada (secreto maestro) ahora? (s/N): ", "عرض المفتاح الخاص (السر الرئيسي) الآن؟ (y/N): ", "Afficher la clé privée (secret maître) maintenant ? (o/N) : ", "现在显示私钥（主密钥）吗？(y/N): ", "Показать закрытый ключ (мастер-секрет) сейчас? (д/N): ", "Özel anahtar (ana sır) şimdi gösterilsin mi? (e/H): ", "کلید خصوصی (راز اصلی) الان نمایش داده شود؟ (y/N): "]),
    ("key_label", ["Private master secret (hex), all keys derive from it:", "Privates Master-Secret (hex), alle Schlüssel leiten sich davon ab:", "Secreto maestro privado (hex), todas las claves derivan de él:", "السر الرئيسي الخاص (hex)، تُشتق منه كل المفاتيح:", "Secret maître privé (hex), toutes les clés en dérivent :", "私有主密钥（hex），所有密钥均由此派生：", "Закрытый мастер-секрет (hex), из него выводятся все ключи:", "Özel ana sır (hex), tüm anahtarlar bundan türetilir:", "راز اصلی خصوصی (hex)، همه‌ی کلیدها از آن ساخته می‌شوند:"]),
    ("lookup_q", ["Pseudonym or public key to look up (Enter = own wallet): ", "Pseudonym oder öffentlicher Schlüssel (Enter = eigene Wallet): ", "Seudónimo o clave pública (Enter = monedero propio): ", "الاسم المستعار أو المفتاح العام (Enter = محفظتي): ", "Pseudonyme ou clé publique (Entrée = mon portefeuille) : ", "要查询的化名或公钥（回车 = 自己的钱包）: ", "Псевдоним или открытый ключ (Enter = свой кошелёк): ", "Aranacak takma ad veya açık anahtar (Enter = kendi cüzdanım): ", "نام مستعار یا کلید عمومی برای جستجو (Enter = کیف پول خودم): "]),
    ("no_coin", ["Note: the Last-change testnet has no coins or on-chain value; holdings are always 0.", "Hinweis: Das Last-change-Testnet hat keine Coins oder On-Chain-Werte; Bestand ist immer 0.", "Nota: la testnet de Last-change no tiene monedas ni valor en cadena; los fondos son siempre 0.", "ملاحظة: شبكة Last-change التجريبية بلا عملات أو قيمة على السلسلة؛ الرصيد دائماً 0.", "Remarque : le testnet Last-change n'a ni pièces ni valeur on-chain ; les avoirs sont toujours 0.", "注意：Last-change 测试网没有币，也没有链上价值；持有量始终为 0。", "Примечание: в тестовой сети Last-change нет монет и ценности в цепи; остаток всегда 0.", "Not: Last-change test ağında coin veya zincir üstü değer yoktur; varlık her zaman 0'dır.", "توجه: شبکه‌ی آزمایشی Last-change هیچ سکه یا ارزش روی زنجیره ندارد؛ موجودی همیشه ۰ است."]),
    ("explorer_info", ["Explorer: run `lastchange explorer` (desktop/full build), then open http://127.0.0.1:8090", "Explorer: `lastchange explorer` starten (Desktop-/Full-Build), dann http://127.0.0.1:8090 öffnen", "Explorador: ejecute `lastchange explorer` (versión completa) y abra http://127.0.0.1:8090", "المستكشف: شغّل `lastchange explorer` (النسخة الكاملة) ثم افتح http://127.0.0.1:8090", "Explorateur : lancez `lastchange explorer` (version complète), puis ouvrez http://127.0.0.1:8090", "浏览器：运行 `lastchange explorer`（完整版），然后打开 http://127.0.0.1:8090", "Обозреватель: запустите `lastchange explorer` (полная сборка) и откройте http://127.0.0.1:8090", "Gezgin: `lastchange explorer` çalıştırın (tam sürüm), sonra http://127.0.0.1:8090 açın", "کاوشگر: `lastchange explorer` را اجرا کنید (نسخه‌ی کامل) و http://127.0.0.1:8090 را باز کنید"]),
    ("send_na", ["Sending is not available: the Last-change testnet has no coins. See docs/FUTURE_POW_TOKEN.md", "Senden ist nicht verfügbar: Das Last-change-Testnet hat keine Coins. Siehe docs/FUTURE_POW_TOKEN.md", "El envío no está disponible: la testnet de Last-change no tiene monedas. Vea docs/FUTURE_POW_TOKEN.md", "الإرسال غير متاح: شبكة Last-change التجريبية بلا عملات. راجع docs/FUTURE_POW_TOKEN.md", "L'envoi n'est pas disponible : le testnet Last-change n'a pas de pièces. Voir docs/FUTURE_POW_TOKEN.md", "无法发送：Last-change 测试网没有币。见 docs/FUTURE_POW_TOKEN.md", "Отправка недоступна: в тестовой сети Last-change нет монет. См. docs/FUTURE_POW_TOKEN.md", "Gönderim kullanılamaz: Last-change test ağında coin yok. Bkz. docs/FUTURE_POW_TOKEN.md", "ارسال در دسترس نیست: شبکه‌ی آزمایشی Last-change سکه ندارد. ببینید docs/FUTURE_POW_TOKEN.md"]),
];

/// Column index for a language code; unknown codes fall back to English.
pub fn idx(code: &str) -> usize {
    LANGS.iter().position(|l| l.0 == code).unwrap_or(0)
}

pub fn t(lang: &str, key: &str) -> &'static str {
    T.iter().find(|r| r.0 == key).expect("i18n key").1[idx(lang)]
}

/// Multilingual header for the language-selection screen (shown before a language is known).
pub const LANG_PROMPT: &str =
    "Language / Sprache / Idioma / اللغة / Langue / 语言 / Язык / Dil / زبان";

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_key_fully_translated() {
        for (k, row) in T {
            assert!(row.iter().all(|s| !s.is_empty()), "{k}");
        }
    }
    #[test]
    fn fallback_and_lookup() {
        assert_eq!(idx("xx"), 0);
        assert_eq!(t("de", "bye"), "Tschüss.");
        assert_eq!(t("fa", "bye"), "خدانگهدار.");
    }
}
