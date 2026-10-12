# Last-change

**بازار همتابه‌همتا و ناشناس، بدون واسطه، با اعتماد قابل اثبات.**

> بیت‌کوین شخص ثالث را از پول حذف کرد. Last-change آن را از خرید و فروش حذف می‌کند.

> ### ⚖️ هشدار حقوقی / Legal
> - این نرم‌افزار **توکن، رمزارز، پرداخت، اسکرو و نگهداری پول ندارد**. پول و کالا خارج از نرم‌افزار و مستقیم بین دو طرف جابه‌جا می‌شود.
> - افزودن توکن، پاداش استخراج، نگهداری دارایی یا سرویس سواپ ممکن است در آلمان و اتحادیه‌ی اروپا فعالیت مجوزدار باشد (**BaFin، KWG، MiCA**). مقررات ضدپول‌شویی اتحادیه‌ی اروپا (**EU AMLR 2024/1624**) هم از **ژوئیه‌ی ۲۰۲۷** ارائه‌دهندگان خدمات رمزارزی را از کار با رمزارزهای افزایندهٔ ناشناسی و حساب‌های ناشناس منع می‌کند. پیش از هر پیاده‌سازی باید از وکیل آلمانی مشاوره گرفت.
> - **خرید و فروش کالا و خدمات غیرقانونی ممنوع است.** ناشناس بودن کسی را از قانون معاف نمی‌کند ([TERMS.md](TERMS.md)).
> - CI با [`scripts/check_no_token.sh`](scripts/check_no_token.sh) تضمین می‌کند که هیچ منطق توکن، استخراج یا پرداختی وارد کد نشود.

## ویژگی‌ها (پیاده‌سازی‌شده)
- **ناشناس و همتابه‌همتا:** هویت فقط یک جفت‌کلید است که روی دستگاه خودتان ساخته می‌شود. نام، ایمیل، تلفن، KYC و پروفایلی وجود ندارد. شبکه با libp2p کار می‌کند (gossipsub، Kademlia و رمزنگاری Noise)، و کلید شبکه جدا از هویت شماست.
- **عرضه و تقاضا:** آگهی کالا، خدمات، و **آثار هنری و نقاشی** (`category=art`) با لینک هش تصاویر. خود تصاویر روی دفتر ذخیره نمی‌شوند.
- **Proof of Trust:** چرخهٔ امضاشدهٔ معامله: پذیرش، «پرداخت شد»، «ارسال شد»، و در آخر تأیید یا اعتراض. اگر فروشنده خریدار را پذیرفته ولی تا مهلت مقرر (پیش‌فرض ۱۴ روز) «ارسال شد» را ثبت نکرده باشد، ادعای عدم تحویل خریدار به او **یک اخطار** می‌دهد. سیاست پیش‌فرض **یک‌اخطاره** است (و قابل تنظیم): آگهی‌های فروشنده نزد گره‌های درستکار پنهان و اعتبارش صفر می‌شود. ادعای دروغ وزن اعتبار خود مدعی را نصف می‌کند، و هر ادعا به مهر ضد اسپم ۱۶ برابر سنگین‌تری نیاز دارد.
- **اعتبار ضد جعل:** امتیاز دادن فقط وقتی ممکن است که فروشنده رسید «پایان معامله» را امضا کرده باشد. وزن هر امتیاز به اعتبار امتیازدهنده بستگی دارد و با گذر زمان کم می‌شود.
- **ضد Sybil و اسپم:** هر پیام به یک مهر کار کوچک نیاز دارد که با **circuit-pow** ساخته می‌شود: از هر seed یک مدار تصادفی محدود ساخته می‌شود (با عملیات صحیح، شاخه‌ها و حافظهٔ scratchpad)، و خروجی‌اش از زنجیرهٔ چندهشی SHA-256، SHA-512، SHA3-256 و BLAKE3 می‌گذرد که ترتیبشان با seed عوض می‌شود. این طراحی در برابر ASIC مقاوم است. سختی با LWMA تنظیم می‌شود. این **استخراج نیست** و هیچ ارزشی تولید نمی‌کند.
- **فیلتر محتوا، فهرست مسدودی و گزارش:** فیلتر محلی، مسدودسازی کلید، شناسهٔ مستعار یا آگهی، و پخش گزارش‌ها در شبکه.
- **API محلی و وب‌اپ:** HTTP/JSON و WebSocket با axum، و یک وب‌اپ فارسی برای ثبت آگهی، مرور، معامله و امتیاز.

## نقشهٔ راه (هنوز پیاده‌سازی نشده)
- 🔜 امضای پساکوانتومی (ML-DSA) و کانال رمزشدهٔ ML-KEM برای پیام‌های معامله
- 🔜 لایهٔ ZK (STARK) برای امتیازدهی ناشناس
- 🔜 اپ اندروید (هستهٔ Rust) و فرانت‌اند web3 که با کلید خود کاربر امضا می‌کند
- 🚫 **توکن EST «اعتیادزدایی جامع»:** اختیاری، استخراجی، با کیف پول پساکوانتومی. **فقط طرح است و پیاده‌سازی نشده** ([docs/FUTURE_POW_TOKEN.md](docs/FUTURE_POW_TOKEN.md)).
- 🚫 **سواپ اتمی Monero و دارایی‌های حذف‌شده از صرافی‌ها:** **فقط طرح است و پیاده‌سازی نشده.** پیش از هر قدمی بررسی حقوقی لازم است.

## شروع سریع
```sh
cargo build --release && B=target/release/lastchange
$B keygen
$B post offer --title "نقاشی رنگ‌روغن" --category art --image <content-hash>
$B serve --http 127.0.0.1:8080          # وب‌اپ: http://127.0.0.1:8080
# گره دوم روی همین کامپیوتر:
$B --dir node2 keygen && $B --dir node2 serve --http 127.0.0.1:8081 --dial /ip4/127.0.0.1/tcp/<port>
./scripts/e2e.sh                         # دمو و تست کامل دوگره‌ای
cargo test --workspace                   # همهٔ تست‌ها
```
فرمان‌های CLI: `keygen whoami post list search accept step claim complete-deal rate trust report block status peers serve`

## ساختار
`crates/circuit-pow` (مهر کار) · `core` (انواع و امضا) · `ledger` (دفتر SQLite با ریشهٔ Merkle) · `reputation` (اعتبار و Proof of Trust) · `net` (libp2p) · `node` (CLI، API و وب‌اپ) · `web/`

## سازنده
**Salar Khanbaba (salnorin)**. همچنین ببینید: [MISSION.md](MISSION.md)

---
### English summary
Last-change is a pseudonymous peer-to-peer classifieds marketplace in Rust. Identities are keys only. It
offers offers/requests (including art), libp2p gossip, a receipt-gated weighted reputation ledger, a
**Proof of Trust** deal lifecycle with a one-strike policy, and anti-spam/anti-Sybil stamps from the
ASIC-resistant `circuit-pow` crate. It also has a content filter, blocklists, gossiped reports, a CLI,
a local HTTP/WebSocket API and a web app.
It has **no token, no payments, no custody**. The EST token and Monero/atomic swaps are design-only roadmap
items and are not implemented. Illegal goods are forbidden, and pseudonymity does not exempt anyone from the law.
See the BaFin/MiCA and EU AMLR (July 2027) notes above. Author: Salar Khanbaba (salnorin).
