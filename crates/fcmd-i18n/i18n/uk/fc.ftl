# Key bar labels: short, they must fit in a few columns.
key-help = Допом
key-menu = КорМн
key-view = Перегл
key-edit = Редаг
key-copy = Копіюв
key-move = Перейм
key-mkdir = Папка
key-delete = Видал
key-pulldown = КонфМн
key-quit = Вихід
# The key bar while a modifier is held: FAR's own words, six columns wide.
key-add = Додати
key-extract = Розпак
key-arccmd = АрхКом
key-edit-new = Редаг…
key-rename = Перейм
key-group = Група
key-selup = Вибран
key-left = Ліва
key-right = Права
key-sort-name = Ім'я
key-sort-ext = Тип
key-sort-write = Запис
key-sort-size = Розмір
key-sort-unsort = Несорт
key-sort-create = Створ.
key-sort-access = Доступ
key-sort = Сорт
key-mklink = Посил.
key-find = Пошук
key-history = Істор
key-folders = ІстПап
key-plugins = Плагін

# Panel chrome
list-up = Угору
list-folder = Папка
col-name = Ім'я
col-ext = Тип
col-size = Розмір
col-allocated = Виділено
col-modified = Змінено
col-created = Створено
col-accessed = Доступ
col-attr = Атр
col-owner = Власник
col-group = Група
col-descr = Опис
col-links = Пос
col-git = Git
descr-title = Опис { $name }
descr-set = { $name }: опис записано
descr-cleared = { $name }: опис прибрано
descr-not-here = описи зберігаються поруч із папкою на цьому комп'ютері
col-date-time = Дата       Час
panel-list-count = елементів у списку: { $count }
panel-empty = (порожньо)
panel-loading = читання…

# Prompts
prompt-select-mask = Позначити файли за маскою:
prompt-deselect-mask = Зняти позначення за маскою:
prompt-goto = Перейти до папки:
prompt-filter = Маска файлів або частина імені:
prompt-search = Швидкий пошук:

# Dialogs
dialog-error = Помилка
dialog-help-hint = Esc повертає до змісту · F7 шукає в усіх розділах · F1 закриває · ↑↓ PgUp PgDn гортають
dialog-help-index = FinalCommander { $version } — Допомога
dialog-help-hint-index = Enter відкриває розділ · F7 шукає в усіх розділах · Esc закриває
help-search-prompt = Пошук у допомозі
help-search-none = Жоден рядок допомоги не містить «{ $query }»
help-search-title = «{ $query }» у допомозі: рядків — { $count }
output-search-prompt = Пошук у виводі
output-search-empty = Ще нічого не виведено
output-search-found = Рядок { $line } з { $lines }
output-search-wrapped = Рядок { $line } з { $lines }, знову від найновішого
output-search-none = Жоден рядок виводу не містить «{ $query }»

# Status messages
msg-not-implemented = { $command } у цій збірці ще не реалізовано
msg-selected = позначено елементів: { $count }
msg-deselected = знято позначення з елементів: { $count }
msg-mask-invalid = неправильна маска: { $error }
msg-theme = тема: { $name }
msg-font-bigger = шрифт: у термінала запитано більший
msg-font-smaller = шрифт: у термінала запитано менший
msg-font-reset = шрифт: у термінала запитано початковий
msg-sort = сортування: { $key }{ $reverse }
msg-sort-groups-on = групи сортування ввімкнено ({ $count })
msg-sort-groups-off = групи сортування вимкнено ({ $count })
msg-sort-groups-none = груп сортування не задано: див. [[sort_group]] у конфігурації
msg-view = вигляд: { $mode }
msg-hidden-on = приховані файли показано
msg-hidden-off = приховані файли сховано
msg-no-history = історії більше немає
msg-tab-last = у панелі завжди є одна вкладка
msg-tab-end = вкладка вже на цьому краю ряду
tabs-title = Вкладки
msg-tab-moved = вкладку перенесено на іншу панель
msg-tab-locked = цю вкладку закріплено за її папкою
msg-tab-locked-on = вкладку закріплено за її папкою
msg-tab-locked-off = вкладку відкріплено
msg-bookmark-set = посилання { $digit }: { $path }
msg-bookmark-unset = посилання { $digit } не задано
msg-bookmark-added = { $path } збережено
msg-bookmark-removed = { $path } прибрано
msg-bookmark-known = ця папка вже є у списку
msg-dirsize = { $path }: { $bytes }, файлів: { $files }
msg-dirsize-many = вимірювання папок: { $count }…
usage-title = Використання диска: { $path } · { $size } · Tab: усе дерево як карта
usage-not-here = використання диска вимірюється для папки на диску, а не в архіві чи списку знайдених файлів
msg-dirsize-no-folder = тут немає папки для вимірювання

# Jobs: prompts, confirmations, progress, conflicts
prompt-copy-to = Копіювати { $what } в:
prompt-move-to = Перейменувати або перемістити { $what } в:
prompt-mkdir = Створити папку:
prompt-tab-name = Назва цієї вкладки:
prompt-bookmark-name = Назва цієї папки:
what-selected = позначені елементи ({ $count })
confirm-delete-title = Видалення
confirm-delete-body =
    { $count ->
        [1] Видалити { $name } ({ $bytes })?
        [one] Видалити { $count } елемент ({ $bytes })?
        [few] Видалити { $count } елементи ({ $bytes })?
       *[other] Видалити { $count } елементів ({ $bytes })?
    }
prompt-link = Створити посилання на { $name } як:
link-created = { $name } створено
confirm-wipe-title = Знищення
confirm-wipe-body =
    { $count ->
        [1] Перезаписати і видалити { $name } ({ $bytes })?
        [one] Перезаписати і видалити { $count } елемент ({ $bytes })?
        [few] Перезаписати і видалити { $count } елементи ({ $bytes })?
       *[other] Перезаписати і видалити { $count } елементів ({ $bytes })?
    }
confirm-wipe-honest = Байти перезаписуються один раз, після чого файл видаляється. На SSD, у файловій системі з копіюванням під час запису або зі знімками старі копії можуть уціліти — а ім'я залишається в каталозі та журналі.
job-copy = Копіювання
job-move = Переміщення
job-delete = Видалення
job-wipe = Знищення
job-trash = Переміщення до кошика
job-replace = Заміна
job-pack = Архівування
job-rebuild = Запис архіву

# ── the attributes dialog (SPEC 10 §3.8) ───────────────────────────────────────────────
attr-title = Атрибути файлу
attr-count = позначено елементів: { $count }
attr-now = зараз { $text } ({ $octal })
attr-owner-read = Власник може читати
attr-owner-write = Власник може записувати
attr-owner-exec = Власник може виконувати
attr-group-read = Група може читати
attr-group-write = Група може записувати
attr-group-exec = Група може виконувати
attr-other-read = Усі можуть читати
attr-other-write = Усі можуть записувати
attr-other-exec = Усі можуть виконувати
attr-setuid = Виконувати від імені власника (setuid)
attr-setgid = Виконувати від імені групи (setgid)
attr-sticky = Видаляти може лише власник (sticky)
attr-readonly = Лише для читання
attr-hidden = Прихований
attr-system = Системний
attr-archive = Архівний
attr-temporary = Тимчасовий
attr-offline = Автономний
attr-indexed = Індексатор може його читати
attr-compressed = стиснений
attr-encrypted = зашифрований
attr-sparse = розріджений
attr-reparse = точка повторного аналізу
attr-also = також { $what }
attr-mtime = Змінено
attr-atime = Останнє читання
attr-btime = Створено
attr-ctime = Запис змінено
attr-btime-readonly = (час створення можна задати лише у Windows)
attr-utc = Час в UTC
attr-owner = Власник (користувач:група)
attr-owner-is = власник { $who }
btn-attr-original = Початкові
btn-attr-current = Поточні
btn-attr-blank = Очистити
attr-recursive = І все, що всередині папок
attr-bad-time = `{ $text }` не є датою і часом (YYYY-MM-DD HH:MM:SS)
attr-bad-owner = тут немає облікового запису чи групи з іменем `{ $text }`
attr-in-archive = ця зміна перезаписує архів заново, решта записів лишається як є
job-attrs = Встановлення атрибутів

# ── the copy dialog (SPEC 03 §4) ───────────────────────────────────────────────────────
copy-title = Копіювання
copy-title-move = Перейменування або переміщення
copy-target = Куди
copy-conflict = Якщо файл уже існує
copy-conflict-ask = запитати
copy-conflict-overwrite = перезаписати
copy-conflict-newer = перезаписати, якщо джерело новіше
copy-conflict-skip = пропустити
copy-conflict-rename = дати копії вільне ім'я
copy-conflict-append = дописати в кінець
copy-only-newer = Лише файли, новіші за наявні
copy-filter = Лише імена, що відповідають масці
copy-follow-links = Копіювати те, на що вказує посилання, а не посилання
copy-keep-times = Зберігати час
copy-keep-mode = Зберігати права доступу
copy-verify = Перечитати кожен файл і порівняти
copy-several = призначень: { $count }, одне за одним

# ── the terminal (SPEC 06 §4) ──────────────────────────────────────────────────────────
term-started = термінал, запущено { $shell }
term-hint = Ctrl+Alt+E: назад до панелей
term-ended = він завершився — будь-яка клавіша закриє це
job-mkdir = Створення папки
job-planning = планування…
job-counters = файлів: { $files } / { $filestotal } · { $bytes } / { $bytestotal }
job-rates = { $speed }/с · минуло { $elapsed } · лишилось { $eta }
job-rates-no-eta = { $speed }/с · минуло { $elapsed }
job-hint = Esc скасовує
job-hint-queue = Esc скасовує · B у фон
job-queued = { $kind } у черзі, незавершених завдань перед ним: { $ahead } · Alt+Shift+F5 показує список
job-background = { $kind } триває у фоні · Alt+Shift+F5 показує завдання
job-asks = { $kind } чекає на відповідь · Alt+Shift+F5
job-paused = { $kind }: пауза
job-resumed = { $kind }: продовжено
job-retry-nothing = Нічого повторювати
job-still-waiting = Це завдання чекає своєї черги
jobs-title = Завдання (незавершених: { $active })
jobs-hint = Enter показати · Space пауза · Del зупинити · F5 повторити · Ctrl+↑↓ порядок · Esc закрити
jobs-quit-title = Завдання ще виконуються
jobs-quit-body = Ці завдання зупиняться, якщо завершити програму зараз:
job-state-running = виконується
job-state-paused = пауза
job-state-asking = запитує
job-state-queued = чекає
job-state-held = затримано
job-state-done = готово
job-state-failed = помилки
job-state-cancelled = скасовано
job-done = { $kind }: файлів: { $files }, { $bytes }
job-done-renamed = { $kind }: файлів переміщено без копіювання: { $files }
job-done-skipped = , пропущено: { $skipped }
job-done-verified = , перевірено: { $verified }
job-done-unverified = , не вдалося перечитати тут: { $unverified }
job-done-errors = , помилок: { $errors }
job-cancelled = { $kind }: скасовано
job-errors-title = Помилки
conflict-title = Призначення вже існує
conflict-dst = призначення  { $path }   { $bytes }
conflict-src = джерело      { $path }   { $bytes }
conflict-newer = джерело новіше
conflict-older = призначення новіше
btn-overwrite = Замінити
btn-overwrite-all = Усі
btn-skip = Пропустити
btn-skip-all = Пропустити всі
btn-retry = Повторити
btn-move = Перемістити
btn-delete = Видалити
btn-wipe = Знищити
btn-copy = Копіювати
btn-discard = Відкинути
btn-kill = Завершити
btn-quit = Вийти
btn-play = Відтворити
btn-run = Виконати
btn-accept = Прийняти
btn-cancel = Скасувати
msg-nothing-to-do = нічого не позначено
cmd-complete-nothing = нема чим це доповнити
complete-several = імен, що так починаються: { $count }
form-moving = стрілки рухають діалог · Enter або Esc залишає його там
form-moved = діалог залишається там, де ви його поставили
msg-trash-refused = Кошик цього не прийняв — Shift+Del видаляє одразу.

# Checksums (SPEC 03 §9)
sum-title = Контрольні суми
sum-algorithm = Хеш
sum-one-file = Один список для всіх
sum-name = Назва
sum-working = { $name }: { $done } з { $total }
sum-verifying = перевірка за { $name }
sum-stopped = зупинено
sum-written = { $name } записано
sum-no-name = списку потрібна назва
sum-local-only = контрольну суму можна порахувати лише для файлу на цьому комп'ютері
sum-all-match = усі { $count } збігаються
sum-verify-title = Контрольні суми
sum-verify-summary = збігаються: { $ok }, не збігаються: { $bad }
sum-differs = не збігається
sum-missing = відсутній
sum-unreadable = не читається

# Synchronising two directories (SPEC 03 §10)
sync-title = Синхронізація
sync-between = { $left }  ↔  { $right }
sync-recursive = Також усе в підпапках
sync-both-ways = В обидва боки, перемагає новіший
sync-delete-extra = Пропонувати прибрати те, чого немає з іншого боку
sync-by-content = Порівнювати байти, а не розмір і час
sync-filter = Лише імена, що відповідають масці
sync-walking = перегляд { $at } — поки що { $pairs }
sync-planned = копіювати: { $copies } ({ $bytes }), прибрати: { $deletes }
sync-hint = Space наступна відповідь · ← → напрямок · = залишити · F2 виконати · Esc скасувати
sync-hint-walking = Esc зупиняє обхід
sync-nothing-to-do = у кожному рядку сказано залишити все як є
sync-local-only = синхронізація може читати й писати лише на цьому комп'ютері
sync-same-directory = обидві панелі в одній папці

# What to run for a kind of file (SPEC 06 §5)
assoc-none = тут ніщо не каже, як відкрити { $name }
usermenu-empty = Меню користувача ще немає: створіть його в { $path }
usermenu-last-gone = рядка, обраного минулого разу, у цьому меню вже немає
usermenu-bad-condition = Умову не зрозуміло — { $problem }
assoc-desktop = Передати робочому столу
assoc-open-with = Відкрити { $name } за допомогою
assoc-no-pager = ні асоціація, ні $PAGER не кажуть, чим це читати
assoc-no-editor = ні асоціація, ні $VISUAL не кажуть, чим це редагувати

# Viewer (F3)
key-wrap = Згорн
key-hex = Код
key-text = Текст
key-dump = Дамп
key-doc = Док
key-search = Пошук
key-code = КодСтр
key-follow = Стежити
viewer-prompt-search = Шукати ({ $mode }):
viewer-prompt-goto = Перейти до зсуву або NN% :
viewer-not-found = `{ $query }` не знайдено
viewer-follow-on = стеження за кінцем файлу
viewer-follow-off = режим стеження вимкнено
viewer-codepage = кодова сторінка: { $name }
viewer-wrap-mode = { $mode }
viewer-no-undo = попередньої позиції немає
viewer-empty = (порожній файл)
viewer-not-markdown = це не файл Markdown, або він завеликий для верстки
viewer-not-table = це не табличний файл, або він завеликий, щоб прочитати його повністю
viewer-base64-done = Декодовано: { $bytes }
viewer-not-base64 = Цей файл не є base64: { $why }
viewer-status = { $mode } { $codepage }  { $offset } / { $len }  { $percent }%

key-disasm = Дизасм
viewer-disasm-failed = не вдається дизасемблювати { $name }: { $error }
viewer-decompile-running = { $tool } працює над { $name }…
viewer-decompile-none = тут нічим декомпілювати { $format }; встановіть інструмент і додайте правило [[decompile]] у fcmd.toml
arc-needs-program = цей архів читає `{ $program }`, якої на цьому комп'ютері не встановлено
viewer-decompile-failed = { $tool }: { $error }
viewer-blame-derived = git blame стосується самого файлу: спершу поверніться до нього
viewer-derived = { $producer } для { $name }

# Editor (F4)
key-save = Зберег
key-replace = Заміна
editor-prompt-goto = Перейти до рядка або рядок:колонка :
editor-prompt-save-as = Зберегти як:
editor-saved = { $name }: збережено, байтів: { $bytes }
editor-nothing-to-save = { $name } не має незбережених змін
editor-not-found = `{ $query }` не знайдено
editor-replaced = замін: { $count }
editor-read-only = { $name } лише для читання
editor-readonly-on = лише для читання: зміни відхиляються
editor-readonly-off = режим «лише для читання» вимкнено
editor-no-undo = більше нічого скасовувати
editor-no-redo = нічого повторювати
editor-discard-title = Незбережені зміни
editor-discard-body = { $name } має незбережені зміни. Вийти і втратити їх?
editor-changed-title = Змінено на диску
editor-changed-body = { $name } змінено на диску після відкриття. Зберегти поверх?
editor-reloaded = { $name } перечитано з диска
editor-failed = { $name }: { $error }

# netbox (SPEC 04 §6)
net-sites-title = З'єднання
net-sites-hint = Enter з'єднати · n URL · Ins зберегти · F4 редагувати · d роз'єднати · Esc закрити
net-no-sites = Сайтів ще немає. Натисніть `n`, щоб увести URL, або додайте блоки [[site]] до конфігурації.
net-not-a-stored-session = цей рядок — відкрите з'єднання, а не збережений сеанс
net-site-forgotten = { $name } більше не зберігається
net-prompt-url = З'єднатися з:
net-prompt-password = Пароль для { $user }@{ $host }:
net-prompt-passphrase = Парольна фраза для ключа { $user }@{ $host }:
net-connecting = з'єднання з { $host }…
net-connected = з'єднано з { $host }
net-disconnected = { $host } роз'єднано
net-not-connected = { $host } не з'єднано
net-local-panel = ця панель на локальній файловій системі
net-nothing-pending = жодне з'єднання цього не чекає
net-hostkey-title = Невідомий ключ вузла
net-hostkey-body = { $host } немає в known_hosts. Його відбиток:
net-hostkey-question = Записати його і з'єднатися?
net-hostkey-changed-title = Ключ вузла ЗМІНИВСЯ
net-hostkey-changed-body = { $host } відповів ключем, який не збігається із записаним.
net-hostkey-changed-where = Записаний — у { $path }, рядок { $line }.
net-hostkey-changed-what = Перевстановлений сервер виглядає точнісінько як хтось посередині. Тут нічого не приймається: відкрийте файл, прочитайте рядок і приберіть його самі, якщо знаєте, чому ключ змінився.
btn-open-file = Відкрити файл
net-fetching = отримання { $name }…
net-viewing-copy = { $name }: локальна копія, отримана для перегляду
net-too-big = { $name } більший за { $limit }: збільште net.fetch_limit_mb, щоб отримати його
net-editing = { $name }: редагується локальна копія; збереження записує її назад
net-writing-back = запис { $name } назад…
net-written-back = { $name }: записано назад, байтів: { $bytes }
net-write-back-failed = записати назад не вдалося: { $error } — ваші зміни в { $local }
net-changed-title = Змінено на сервері
net-changed-body = { $name } змінено на сервері після отримання. Записати поверх?
confirm-delete-permanent = Це видаляє остаточно: кошика для цього немає.
confirm-trash-title = Переміщення до кошика
confirm-trash-body =
    { $count ->
        [1] Перемістити { $name } ({ $bytes }) до кошика?
        [one] Перемістити { $count } елемент ({ $bytes }) до кошика?
        [few] Перемістити { $count } елементи ({ $bytes }) до кошика?
       *[other] Перемістити { $count } елементів ({ $bytes }) до кошика?
    }
confirm-trash-where = Вони потрапляють туди, де робочий стіл тримає видалені файли, і їх можна звідти повернути.

# ---------------------------------------------------------------- menu bar (SPEC 10)
menu-left = &Ліва
menu-right = &Права
menu-files = &Файли
menu-commands = &Команди
menu-options = Па&раметри
menu-brief = &Короткий
menu-medium = &Середній
menu-full = &Повний
menu-detailed = &Детальний
menu-owners = Вл&асники файлів
menu-links = Посилання файлів
menu-alt-full = Повний, розміри повніс&тю
menu-descriptions = &Описи
menu-long-descriptions = До&вгі описи
menu-wide = &Широкий
menu-info-panel = Панель ін&формації
menu-quick-view = Швидки&й перегляд
menu-tree-panel = Де&рево папок
menu-tab-new = &Нова вкладка
menu-tab-close = Закрити вкладку
menu-tab-next = Наст&упна вкладка
menu-tab-prev = Попередн&я вкладка
menu-tab-move = Перенести вкладку на іншу панел&ь
menu-tab-lock = Закріпити вкладку за її папко&ю
menu-tab-name = Назвати цю вкладку
menu-tab-list = Усі вкладки
menu-sort-modes = Режим&и сортування
menu-hidden = Приховані та системні файли
menu-filter = Фі&льтр файлів
menu-filter-sets = Іменовані фільтри
menu-panel-onoff = Панель ув&імк/вимк
menu-reread = П&еречитати
menu-change-drive = З&мінити диск
menu-view = &Перегляд
menu-edit = &Редагування
menu-edit-new = Редагувати новий &файл
menu-copy = &Копіювання
menu-rename = П&ерейменування або переміщення
menu-link = Створити поси&лання
menu-multirename = Перейменувати за &шаблоном…
menu-multirename-undo = Скасувати останнє групове перейменування
menu-mkdir = &Створення папки
menu-delete = &Видалення
menu-wipe = Зни&щення
menu-checksum = Контрольні с&уми…
menu-checksum-verify = Перевірити за списком
menu-split = Розрізати файл…
menu-combine = Зібрати файли
menu-delete-permanent = Видалити остато&чно
menu-archive-add = &Архівувати
menu-archive-extract = Р&озпакувати
menu-archive-commands = Архівн&і команди
menu-archive-test = Перевірити архів
menu-attributes = А&трибути файлів
menu-apply-command = Застосувати коман&ду
menu-replace-in-files = Заміна у файлах…
menu-select-group = Позначити &групу
menu-select-ext = Позначити за цим т&ипом
menu-unselect-ext = Зняти позначення за цим типо&м
menu-unselect-group = З&няти позначення
menu-invert-selection = Інверсі&я позначення
menu-select-all = Позначити все
menu-restore-selection = Відновити по&значення
menu-dirsize = Розмір папки
menu-find-file = &Пошук файлу
menu-history = &Історія команд
menu-view-history = Іс&торія перегляду й редагування
menu-folders-history = Іст&орія папок
menu-usage = Використання &диска
menu-jobs = &Завдання
menu-branch = Плоский вигляд гілки
menu-bookmarks = Посилання на папки
menu-swap-panels = По&міняти панелі
menu-sync-panel = &Синхронізувати іншу панель
menu-panels-onoff = Панелі у&вімк/вимк
menu-compare-folders = Порівн&яння папок
menu-sync-folders = Син&хронізація папок
menu-compare-contents = Порів&няти за вмістом
menu-panelize = Вивід команди на панель
menu-view-filtered = Перегляд через команду
menu-learn-keys = Вивчити клавіші
menu-tunnel = Від&крити тунель…
menu-tunnels = Тун&елі…
tunnel-title = Тунель: [локальний:]вузол:порт
tunnel-list-title = Тунелі
tunnel-none = жодного тунелю не відкрито
tunnel-open = тунель відкрито: { $what }
tunnel-closed = тунель закрито
tunnel-failed = { $what }: { $why }
tunnel-needs-connection = тунель іде через з'єднання: спершу відкрийте його через Alt+N
menu-connections = З'єднання
menu-disconnect = Роз'єднати
menu-user-menu = Меню користува&ча
menu-associations = &Асоціації файлів
menu-folder-shortcuts = Поси&лання на папки
menu-plugin-commands = Команди пла&гінів
menu-screens-list = Список ек&ранів
menu-macro-record = Записати макрос
menu-macro-list = Макроси
menu-task-list = Список задач
menu-system-settings = Си&стемні параметри
menu-panel-settings = Налаштування па&нелі
menu-interface-settings = Налаштування &інтерфейсу
menu-languages = &Мови
menu-themes = &Теми
menu-confirmations = &Підтвердження
menu-viewer-settings = Налаштування про&грами перегляду
menu-editor-settings = Налаштування &редактора
menu-codepages = Ко&дові сторінки
menu-sort-groups = Групи с&ортування
menu-mask-groups = Маски &файлів
menu-settings-all = &Усі параметри
menu-highlighting = Розмал&ьовка файлів
menu-plugins-config = Параметри плагіні&в
menu-help = Допомога
menu-save-setup = &Зберегти параметри
menu-quit = Ви&хід
opt-natural-sort = Природний порядок чисел
opt-allow-reverse = Дозволити зворотне сортування
opt-select-folders = Маски позначають і папки
cmd-no-network-name = Шлях на цьому диску не має мережевого імені
opt-trash = Використовувати системний Кошик
opt-system-copy = Використовувати системну функцію копіювання
opt-save-history = Зберігати історію між сеансами
opt-history-lifetime = Строк зберігання історії, у днях
opt-forever = завжди
opt-fetch-limit = Обмеження розміру віддалених файлів
opt-keepalive = SSH keep-alive
opt-off = вимкнено
opt-show-hidden = Показувати приховані та системні файли
opt-column-titles = Показувати заголовки колонок
opt-status-line = Показувати рядок статусу
opt-scrollbar = Показувати смужку прокручування
opt-free-space = Показувати вільне місце
opt-auto-refresh = Оновлювати панель автоматично
opt-dirs-first = Папки спочатку
opt-selected-first = Позначені файли спочатку
opt-reverse = Зворотний порядок
opt-view = Вигляд панелі
opt-sort = Критерій сортування
opt-quick-search = Швидкий пошук
opt-quick-alt = з Alt
opt-quick-plain = за літерами
opt-key-bar = Показувати рядок клавіш
opt-status-bar = Показувати рядок стану
opt-syntax = Розфарбовувати розпізнані мови
opt-markdown = Markdown, зверстаний для читання
opt-table = Відкривати .csv і .jsonl як таблицю
opt-keymap = Розкладка клавіш
opt-clock = Годинник у кутку
opt-mouse = Підтримка миші
opt-clipboard = Дозволити терміналу передавати буфер обміну (OSC 52)
opt-command-screen = Показувати вивід команди під час її роботи
opt-menu-always = Завжди показувати рядок меню
opt-save-exit = Зберігати параметри під час виходу
opt-confirm-delete = Видалення
opt-confirm-overwrite = Перезапис
opt-confirm-exit = Вихід
opt-confirm-editor = Вихід із редактора з незбереженими змінами
opt-applied = параметри застосовано
opt-saved = збережено в { $path }
opt-set-title = { $key }
opt-set = { $key } = { $value }
opt-not-a-toggle = Це не параметр «так чи ні»: Enter дозволяє ввести значення
opt-settings-hint = ↑↓ вибір · друкуйте для звуження · Enter або F4 значення · Space перемкнути · Del типове · Esc закрити
opt-language-set = мова: { $name }
sort-name = Ім'я
sort-ext = Тип файлу
sort-size = Розмір
sort-mtime = Час запису
sort-btime = Час створення
sort-atime = Час доступу
sort-descr = Опис
sort-owner = Власник
sort-git = Стан Git
key-sort-descr = Опис
key-sort-owner = Власн.
sort-none = Не сортувати
sort-set = сортування: { $name }
btn-ok = OK
pick-hint = ↑↓ вибір · друкуйте для фільтра · Enter застосувати · Esc закрити
pick-empty = (нема з чого вибирати)
# ------------------------------------------------- the three histories (SPEC 06 §6)
hist-pinned = { $line } закріплено · тепер ніщо його не витіснить
hist-unpinned = { $line } більше не закріплено
hist-is-pinned = { $line } закріплено · Ins спершу відкріплює
hist-clear-body = Очистити цю історію від усього, що не закріплено?
hist-cleared = в історії лишилося тільки закріплене
history-hint = ↑↓ вибір · Enter застосувати · Ins закріпити · Shift+Del один · Del усі · Esc закрити


# ------------------------------------------------- session editor (SPEC 10 §3.5)
site-new = Нове з'єднання
site-edit = Редагування з'єднання
site-name = Ім'я
site-host = Вузол
site-port = Порт
site-user = Користувач
link-title = Створення посилання
link-name = Посилання
link-kind = Вид
link-kind-relative = символічне, відносне — переживає перенесення дерева
link-kind-absolute = символічне, абсолютне — переживає перенесення посилання
link-kind-hard = жорстке — друге ім'я того самого файлу
menu-colors = Ко&льори…
palette-title = Кольори { $theme }
palette-hint = Enter колір · F2 зберегти · Esc закрити
palette-ask = Колір для { $key }
palette-bad-color = { $value } не є кольором (назва, 0–255 або #rrggbb)
palette-saved = кольори записано в тему { $theme }
palette-unsaved = кольори змінено лише на екрані — F2 їх записує
opt-viewer-status-line = Показувати верхній рядок (ім'я та позиція)
opt-viewer-scrollbar = Показувати смужку прокручування
opt-viewer-key-bar = Показувати рядок клавіш
opt-viewer-wrap = Довгі рядки
opt-viewer-wrap-off = не згортати
opt-viewer-wrap-char = згортати на краю
opt-viewer-wrap-word = згортати за словами
opt-viewer-tab-width = Розмір табуляції
opt-viewer-remember = Пам'ятати позицію та кодову сторінку кожного файлу
opt-editor-tabs = Табуляції
opt-editor-tabs-keep = залишати як є
opt-editor-tabs-typed = розгортати те, що вводиться
opt-editor-tabs-all = розгортати всі під час відкриття файлу
opt-editor-tab-width = Розмір табуляції
opt-editor-auto-indent = Автовідступ
opt-editor-whitespace = Показувати пробіли й табуляції
opt-editor-cursor-after = Курсор стає після тексту, знайденого пошуком
opt-editor-beyond-eol = Курсор може виходити за кінець рядка
opt-editor-trim-trailing = Прибирати пробіли в кінці рядків під час збереження
editor-not-a-character = ця клавіша не має символу для вставки
paste-one-line =
    { $count ->
        [one] вставлено лише перший рядок · ще { $count } рядок пропущено
        [few] вставлено лише перший рядок · ще { $count } рядки пропущено
       *[other] вставлено лише перший рядок · ще { $count } рядків пропущено
    }
clip-names =
    { $count ->
        [one] { $count } ім'я в буфері обміну
        [few] { $count } імені в буфері обміну
       *[other] { $count } імен у буфері обміну
    }
clip-paths =
    { $count ->
        [one] { $count } шлях у буфері обміну
        [few] { $count } шляхи в буфері обміну
       *[other] { $count } шляхів у буфері обміну
    }
clip-line = командний рядок у буфері обміну
clip-selection = виділене — у буфері обміну
clip-off = буфер обміну вимкнено — `ui.clipboard` у параметрах
history-gone = { $name } уже немає
hl-new = Нова група кольорів
hl-edit = Група кольорів
# ------------------------------------------------- the diff screen (SPEC 05 §3)
diff-title = Відмінності
menu-diff = Показати відмінності &файлів
diff-needs-two = Поставте курсор на файл у кожній панелі
diff-not-here = { $name } не є файлом на цьому диску
diff-too-big = { $name } більший за { $most }
diff-same = Обидва файли однакові
diff-where = блок { $at } з { $count } · рядків: -{ $removed } +{ $added }
diff-no-next = Остання відмінність
diff-no-previous = Перша відмінність
diff-hint = Alt+↑↓ блоки · Tab бік · F5 взяти · F2 записати · U єдиний · Shift+F2 патч · W C E ігнорувати · Esc
diff-hint-git = Alt+↑↓ блоки · S додати блок до індексу · Tab бік · F5 взяти · F2 записати · Esc закрити
diff-hint-git-staged = Alt+↑↓ блоки · S прибрати блок з індексу · Tab бік · F5 взяти · F2 записати · Esc закрити
diff-stage-edited = у порівнянні є незаписані злиття: спершу F2, потім S
diff-merged-left = Лівий файл узяв цю відмінність
diff-merged-right = Правий файл узяв цю відмінність
diff-no-hunk-here = Угорі екрана немає відмінності — Alt+↓ підведе до неї
diff-nothing-to-write = Нічого не злито
diff-written = записано: { $count }
diff-unwritten = Злиття не записано — F2 записує, Esc ще раз залишає як є
diff-ignore-space-on = Пробіли ігноруються
diff-ignore-space-off = Пробіли знову враховуються
diff-ignore-case-on = Регістр ігнорується
diff-ignore-case-off = Регістр знову враховується
diff-ignore-eol-on = Кінці рядків ігноруються
diff-ignore-eol-off = Кінці рядків знову враховуються

grab-hint = ↑↓←→ рух · Shift+↑↓←→ позначити · Ctrl+A усе · Enter копіювати · Esc закрити
grab-copied = скопійовано рядків екрана: { $rows }

# ------------------------------------------------- the code pages (SPEC 05 §1.12)
cp-title = Кодові сторінки
cp-name-title = Як назвати цю кодову сторінку
cp-set = Читається як { $name }
menu-panel-names = Кодова сторінка імен…
names-title = Читати імена як
names-system = Як їх дає система
names-set = Імена читаються як { $name }
names-system-set = Імена такі, як їх дає система
names-not-here = У цій системі ім'я не є байтами, тож іншого способу його прочитати немає
cp-marked = { $name } тепер серед тих, які перебирає F8
cp-unmarked = { $name } більше не серед тих, які перебирає F8
cp-hint = ↑↓ вибір · Enter читати так · Ins позначити для F8 · F4 перейменувати · Esc закрити

# ------------------------------------------------- the named mask groups (SPEC 02 §5b)
mg-title = Група масок
mg-name = Ім'я
mg-mask = Маски
mg-hint = Ins додає групу, Del прибирає, F4 редагує · `·` позначає три вбудовані
mg-saved = <{ $name }> діє; «Зберегти параметри» її запише
mg-removed = <{ $name }> прибрано
mg-bad = <{ $name }>: { $problem }
mg-needs-name = ім'я без пробілів, ком і кутових дужок
mg-needs-mask = група, що нічого не означає, нічому не відповідає
mg-built-in = <{ $name }> — одна з трьох вбудованих · F4 дає їй інші маски

sg-title = Група сортування
sg-name = Ім'я
sg-mask = Маски
sg-place = Де
sg-place-front = перед рештою
sg-place-back = після решти
sg-hint = Ins додає групу, Del прибирає, F4 редагує, Ctrl+↑↓ пересуває
sg-saved = Група діє; «Зберегти параметри» її запише
sg-removed = { $name } прибрано
sg-bad = { $name }: { $problem }
sg-needs-mask = група без маски нічого б не містила
hl-name = Ім'я
hl-mask = Маска або маски
hl-kind = Застосовується до
hl-kind-both = файлів і папок
hl-kind-file = лише файлів
hl-kind-dir = лише папок
hl-size = Розмір (1M-10M, -4K, 1G-)
hl-age = Вік у днях (<=1, >=30, 7-30)
hl-date-field = Вік рахується за
hl-attrs = Атрибути (h r x s b, - означає «не повинно бути»)
hl-fg = Колір тексту (#d7875f, red, file.archive)
hl-bg = Тло, записується так само
hl-hint = ↑↓ вибір · Enter/F4 редагувати · Ins нова · Del прибрати · Ctrl+↑↓ пересунути · Esc закрити
hl-saved = група діє · Параметри → Зберегти параметри запише її у файл
hl-removed = { $name } прибрано · Параметри → Зберегти параметри запише це у файл
hl-bad = { $name }: { $problem }
filter-off = Без фільтра: усі імена
filter-hint = ↑↓ вибір · Enter застосувати · F4 редагувати · Ins новий · Del прибрати · Esc закрити
filter-new = Новий фільтр
filter-edit = Фільтр
filter-name-hint = з іменем він зберігається в меню фільтрів; без імені використовується один раз
filter-applied = фільтр увімкнено: { $name }
filter-cleared = фільтр вимкнено
filter-saved = { $name } діє на панелі та є в меню фільтрів · Параметри → Зберегти параметри запише його у файл
filter-removed = { $name } прибрано · Параметри → Зберегти параметри запише це у файл
editor-no-block = нічого не виділено
editor-unique = прибрано повторних рядків: { $count }
editor-no-bracket = тут немає дужки, або їй немає пари
editor-tabs-expanded = табуляції розгорнуто в рядках: { $lines } — збереження це запише
site-protocol = Протокол
site-protocol-sftp = SFTP (через SSH)
site-protocol-ftp = FTP
site-protocol-ftps = FTP з TLS
site-protocol-dav = WebDAV
site-protocol-davs = WebDAV через HTTPS
site-auth = Автентифікація
site-auth-agent = агент SSH
site-auth-key = файл ключа
site-auth-ask = запитати пароль
site-key-file = Файл ключа
site-start-dir = Віддалена папка
site-group = Група
site-keepalive = Keep-alive (с)
site-compress = Стиснення
site-page-session = Сеанс
site-page-environment = Середовище
site-page-directories = Папки
site-page-sftp = SFTP
site-password-note = Пароль не зберігається: його буде запитано під час з'єднання.
site-env-eol = Перетворювати кінці рядків (ще ні)
site-env-utf8 = Примусово імена в UTF-8 (ще ні)
site-jump = Проміжний вузол
site-jump-note = З'єднання йде через цей вузол: host, user@host або user@host:port. Порожньо — одразу до сервера.
site-sftp-ciphers = Вибрати шифри (ще ні)
btn-connect = З'єднати
site-needs-host = з'єднанню потрібен вузол
site-needs-key = виберіть файл ключа або інший спосіб автентифікації
site-bad-port = `{ $port }` не є числом
site-stored = з'єднання збережено: { $name } (Shift+F9, щоб записати)
# ------------------------------------------------- structure pane (SPEC 05 §1.8)
key-struct = Структ
key-markdown = MD
key-table = Табл
key-filter = Фільтр
key-decompile = Декомп
key-next = Далі
key-prev = Назад
key-codepages = КодСтр
key-goto = Перейт
key-to-panel = Панель
key-blame = Blame
viewer-structure = Структура
viewer-structure-hint = ↑↓ рядок · Enter перейти · Tab вміст · Shift+F11 закрити
viewer-structure-hint-tab = Tab переводить клавіші сюди
viewer-no-structure = нічого декодувати: панель структури не знає цього формату
viewer-no-target = цей рядок нікуди у файлі не вказує
viewer-form-opened = Форма у вигляді тексту, яким її показує Delphi. F4 повертає до байтів.
viewer-metadata-opened = Увесь текст, зверстаний для читання. F4 повертає до байтів.
viewer-went-to = перехід до { $offset }
viewer-in-panel = панель стоїть на { $name }
viewer-not-in-panel = { $name } немає в цій панелі
viewer-no-more-files = у панелі більше немає файлів
editor-unsaved-for-viewer = спершу збережіть: переглядач читає файл із диска
# ------------------------------------------------- command line (SPEC 06 §7)
cmd-running = виконується: { $command }
prompt-panelize = Команда, вивід якої називає файли
prompt-viewer-filter = Команда, вивід якої читати замість файлу
learn-title = Які клавіші надсилає цей термінал
learn-hint = Натискайте клавіші. Підсвічена клітинка означає, що клавіша дійшла; Esc двічі — вихід.
learn-last = остання: { $key } — { $command }
learn-unbound = ні до чого не прив'язана
recode-title = Перезаписати як
recode-what = файлів: { $count }
recode-from = Чим вони є зараз
recode-to = Записати їх як
recode-auto = визначати для кожного файлу
recode-eol = Кінці рядків
recode-eol-keep = не чіпати
recode-working = перезапис файлів: { $count }
recode-done = записано: { $changed }, уже такі, як треба: { $same }, відхилено: { $failed }
menu-recode = Перезаписати як…
panelize-title = { $command }
panelize-done = файлів: { $count }, пропущено рядків: { $dropped }
panelize-nothing = { $command }: жоден рядок не назвав наявного файлу
cmd-interrupted = команді надіслано переривання
cmd-done = { $command } завершено
cmd-failed = { $command } завершилась із кодом { $code }
cmd-busy = команда вже виконується
prompt-apply = Команда для кожного з файлів ({ $count }):
cmd-apply-no-ask = команду, що запитує значення, не можна застосувати до файлів по одному
cmd-apply-progress = { $command }  ({ $done } з { $total })
cmd-apply-stopped = зупинено після { $done } з { $total }: { $command } завершилась помилкою
cmd-not-remote = команді потрібна локальна папка; ця панель на сервері
cmd-no-history = історія команд порожня
cmd-no-previous = нема куди повертатися
cmd-no-such-dir = { $path }: такої папки немає
cmd-no-such-path = { $path }: такого файлу чи папки немає
cmd-no-selection = нічого не позначено
cmd-condition-false = не виконано: { $command }
cmd-screen-cleared = екран очищено
cmd-stack-empty = стек папок порожній
cmd-stack-cleared = стек папок очищено
cmd-pushed = папок у стеку: { $count }
cmd-environment = середовище
cmd-set = { $name } = { $value }
cmd-not-set = { $name } не задано
user-screen-title = Вивід — рядків: { $lines }, з рядка { $at }
user-screen-empty = (ще нічого не запускалося)
viewer-no-branch = у цьому рядку немає адреси для переходу
viewer-no-back = нема куди повертатися

# ------------------------------------------------- find files (SPEC 06 §1)
folders-title = Пошук папки
folders-reading = Читання папок…
folders-read = папок: { $count }
folders-capped = папок: { $count }, до межі
folders-local-only = список папок стосується цього комп'ютера; сервер має свою історію та список посилань
find-title = Пошук файлу
find-mask = Маска або маски файлів
find-size = Розмір (100M, 1M-1G, -4K)
find-text = Містить текст
find-case = Враховувати регістр
find-words = Лише цілі слова
find-hex = Текст вище — шістнадцятковий шаблон
find-not = Файли, які його НЕ містять
find-scope = Шукати в
find-scope-panel = цій папці
find-scope-tree = цій папці та нижче
find-scope-selection = позначених папках
find-scope-root = від кореня цього диска
find-scope-other = іншій панелі та нижче
find-links = Переходити за символічними посиланнями
find-dirs = Папки теж у результатах
find-archives = Шукати в архівах
find-results = Знайдено { $count } · переглянуто файлів: { $files }, папок: { $dirs }
find-done = знайдено { $count }, файлів переглянуто: { $files } · { $elapsed }
find-none = нічого не знайдено
find-stopped = зупинено після перегляду файлів: { $files }
find-hint = ↑↓ вибір · Enter перейти · F3 перегляд · F4 редагувати · F5 на панель · Esc зупинити
find-hint-done = ↑↓ вибір · Enter перейти · F3 перегляд · F4 редагувати · F5 на панель · Esc закрити
find-panelised = результатів на панелі: { $count }
find-needs-mask = потрібна маска: `*` означає всі файли
find-bad-size = `{ $text }` не є розміром — { $error }
# ------------------------------------------------- replace across files
replace-title = Заміна у файлах
replace-needle = Що шукати
replace-with = Замінити на
replace-case = Враховувати регістр
replace-words = Лише цілі слова
replace-pattern = Текст вище — регулярний вираз
replace-hex = Обидва — шістнадцяткові байти
replace-scope = позначено файлів: { $count }
replace-needs-needle = нема чого шукати
replace-confirm-title = Заміна
replace-confirm-body = Буде змінено входжень: { $count }, у файлах: { $files }.
replace-nothing = нічого замінювати: жоден файл не зміниться
replace-done = замінено входжень: { $count }, у файлах: { $files }

# ── archives (SPEC 04 §4) ───────────────────────────────────────────────────────────────
arc-writing-back = Запис { $name } назад в архів
arc-move-out = щоб забрати щось з архіву, скопіюйте це через F5
arc-add-files-only = по одному додаються лише файли; Shift+F1 додає цілу папку

rename-title = Групове перейменування
rename-name = Ім'я
rename-ext = Тип файлу
rename-search = Шукати
rename-replace = Замінити на
rename-regex = Регулярний вираз ($1 у заміні)
rename-case = Літери
rename-case-same = як є
rename-case-lower = малі літери
rename-case-upper = ВЕЛИКІ ЛІТЕРИ
rename-case-title = Кожне Слово З Великої
rename-case-sentence = Лише перша літера велика
rename-counter = Лічильник: початок, крок, цифри
rename-fields = [N] ім'я  [E] тип  [N1-3] частина  [C] лічильник  [d] [t] дата, час  [P] папка
rename-not-in-archive = кілька файлів перейменовуються в папці, а не в архіві: там F6 перейменовує один запис
rename-bad-counter = лічильник — це три цілі числа: початок, крок, цифри; { $counter } ними не є
rename-cannot = не вдається перейменувати: { $why }
rename-problem-empty = імені не лишилося
rename-problem-invalid = файл не може мати такого імені
rename-problem-twice = це ім'я дістається ще одному файлу
rename-problem-exists = файл із таким іменем уже є
rename-more = … і ще { $count }
rename-blocked =
    { $count ->
        [one] { $count } файл не може отримати нове ім'я: нічого не перейменовується, доки це не виправлено
        [few] { $count } файли не можуть отримати нові імена: нічого не перейменовується, доки це не виправлено
       *[other] { $count } файлів не можуть отримати нові імена: нічого не перейменовується, доки це не виправлено
    }
rename-summary = змінюється { $count } з { $total }
rename-nothing = жодне ім'я не змінюється
rename-nothing-to-undo = немає групового перейменування, яке можна скасувати
rename-done =
    { $count ->
        [one] перейменовано { $count } файл · скасувати можна з меню Файли
        [few] перейменовано { $count } файли · скасувати можна з меню Файли
       *[other] перейменовано { $count } файлів · скасувати можна з меню Файли
    }
rename-undone =
    { $count ->
        [one] повернуто { $count } ім'я
        [few] повернуто { $count } імені
       *[other] повернуто { $count } імен
    }
pack-title = Архівування
pack-name = Архів
pack-format = Формат
pack-level = Стиснення
pack-solid = Суцільні блоки (7z: менший, але один файл читається повільніше)
pack-delete-after = Видалити файли після архівування
pack-level-normal = звичайне
pack-level-fast = швидке
pack-level-best = найкраще
pack-scope = позначено файлів: { $count }
pack-needs-name = архіву потрібне ім'я
pack-format-read-only = { $format } тут лише читається, а не записується: виберіть інший формат або інше ім'я
pack-single-one-file = { $format } містить один файл: виберіть один або візьміть { $tar } для папки чи кількох файлів
unpack-title = Розпакування архіву
unpack-to = Розпакувати в
unpack-scope = з { $name }
unpack-not-an-archive = це не архів, який ми вміємо читати

# ── the special panel kinds (SPEC 02 §7) ────────────────────────────────────────────────
panel-quick-view = Швидкий перегляд
panel-info = Інформація
panel-tree = Дерево
compare-done = відрізняються: { $left } ліворуч, { $right } праворуч
cmp-subdirs = Заходити в підпапки
cmp-depth = Як глибоко заходити (0 = до самого низу)
cmp-trees-title = Що відрізняється
cmp-trees-working = порівняння дерев: файлів { $seen }, прочитано { $read }
cmp-trees-done = відрізняються: { $differ }, лише ліворуч: { $left }, лише праворуч: { $right }, усього пар: { $pairs }
cmp-trees-same = обидва дерева однакові (пар: { $pairs }, перевірено: { $same })
menu-duplicates = Пошук дублікатів
dup-title = Файли-дублікати
dup-working = пошук дублікатів: файлів { $seen }, прочитано { $hashed }
dup-done = груп копій: { $groups }, файлів: { $files }, змарновано { $wasted }
dup-none = тут ніщо не є копією чогось іншого (файлів: { $seen })
dup-stopped = зупинено після перегляду файлів: { $seen }
dup-partial = зупинено достроково
cmp-title = Порівняння
cmp-case = Зіставляти імена з урахуванням регістру
cmp-size = Інший розмір — це відмінність
cmp-time = Інший час — це відмінність
cmp-seconds = Час із різницею до стількох секунд вважається однаковим
cmp-contents = Читати байти
cmp-when-time = Читати їх лише там, де час відрізняється
cmp-fragment = Читати лише стільки перших КіБ (0 = усе)
cmp-ignore-eol = Самі лише кінці рядків не є відмінністю
cmp-selected = Лише позначене
cmp-stop = Зупинитися на першій відмінності
cmp-newer = Позначати лише новішу копію
menu-compare-setup = Порівняння з параметрам&и
compare-contents-eol-ignored = відрізняються лише кінцями рядків, проігноровано: { $count }
compare-contents-trusted = визнано однаковими без читання: { $count }
compare-contents-working = Порівняння вмісту: { $done } з { $total }
compare-contents-done = відрізняються пар: { $differ } з { $pairs }
compare-contents-same = усі пари ({ $pairs }) ідентичні
compare-contents-none = жоден файл не має однакового імені з обох боків
compare-contents-local = для порівняння за вмістом обидві панелі мають бути на локальній файловій системі
compare-contents-eol = лише кінці рядків: { $count }
compare-contents-only = лише з одного боку: { $count }
compare-contents-unread = не вдалося прочитати: { $count }
compare-contents-stopped = порівняння зупинено
quick-view-directory = { $name } — папка: F3 її вимірює
quick-view-not-here = байтів тут немає: F3 спершу їх дістане
info-path = Шлях
info-kind = Вид
info-kind-local = папка на цьому комп'ютері
info-kind-remote = на { $host }
info-kind-archive = усередині { $name }
info-kind-list = список файлів
info-device = Пристрій
info-folder = Папка
info-device-value = { $total }, вільно { $free }, кластери { $cluster }
info-slack = Втрати на кластерах
info-filesystem = Файлова система
info-mount = Змонтовано
info-git = Репозиторій
info-git-clean = як в індексі
info-git-differing = { $count } з { $tracked } відрізняються від індексу
info-git-uncounted = не пораховано
info-git-merge = триває злиття
info-git-rebase = триває rebase
info-git-cherry-pick = триває cherry-pick
info-git-revert = триває revert
info-git-bisect = триває bisect
info-memory = Пам'ять
info-memory-value = вільно { $free } з { $total }
info-memory-total = { $total }
info-contents = Містить
info-contents-value = файлів: { $files }, папок: { $dirs }, { $bytes }
info-selected = Позначено
info-selected-value = файлів: { $count }, { $bytes }
info-cursor = Курсор
info-size = Розмір
info-modified = Змінено
info-description = Опис
key-mark = Позн
key-asm = Асм
key-block = Блок
key-screens = Екрани
key-site-edit = Редаг
key-site-new = Новий
key-site-forget = Видал
key-ai-edit = Редаг
key-ai-new = Новий
key-ai-remove = Видал
key-ai-forget = Забути
key-ai-threads = Історія
hex-needs-a-file = цей буфер не має файлу, з якого читати байти
hex-save-first = спершу збережіть: два редактори не можуть тримати незбережені зміни один одного

# ── the hex editor's block operations (SPEC 05 §2.6) ────────────────────────────────────
hex-block-title = Блок
hex-op-write = Записати { $size } у файл
hex-op-read = Прочитати файл у позицію { $at }
hex-op-insert = Вставити нульові байти в позицію { $at }
hex-op-copy = Скопіювати байти з іншого місця в { $at }
hex-op-move = Перемістити байти з іншого місця в { $at }
hex-op-fill = Заповнити { $size } байтом
hex-op-xor = Застосувати виключне АБО з ключем до { $size }
hex-op-delete = Видалити { $size } — { $was } стане { $becomes }
hex-write-title = Записати блок у
hex-read-title = Який файл прочитати
hex-fill-title = Яким байтом заповнити (hex)
hex-xor-title = Виключне АБО з яким ключем (hex)
hex-insert-title = Скільки нульових байтів
hex-copy-title = Звідки копіювати — зсув і довжина (hex)
hex-move-title = Звідки перемістити — зсув і довжина (hex)
hex-not-hex = це не послідовність шістнадцяткових цифр
hex-not-a-count = це не кількість байтів
hex-not-a-range = це не зсув і довжина в hex
hex-range-outside = цей діапазон поза файлом ({ $size })
hex-move-into-itself = блок не можна перемістити всередину самого себе
hex-filled = заповнено { $size }
hex-xored = до { $size } застосовано виключне АБО — це не шифрування
hex-written = { $size } записано в { $name }
hex-read-done = { $size } прочитано з { $name }
hex-inserted = вставлено нулів: { $size }
hex-deleted = видалено { $size }
hex-device = { $name }: пристрій розміром { $size }, відкрито лише для читання
asm-title = Асемблювати в { $at } ({ $arch })
asm-no-arch = { $arch } не є архітектурою, для якої ми вміємо асемблювати
asm-written = { $text } — { $bytes }
asm-written-longer = { $text } — { $bytes } (на { $grew } байт довше)
screens-title = Екрани
screens-panels = Панелі
quit-title = Вихід із FinalCommander
quit-body = Вийти?
screens-quit-title = Незбережені зміни
screens-quit-body = Ці екрани містять зміни, яких ніхто не записав:
macro-recording = Запис макросу · Ctrl+. зупиняє його
macro-empty = нічого не записано
macro-dropped = запис відкинуто
macro-key-title = Яка клавіша
macro-key-body = Записано кроків: { $steps }. Натисніть клавішу, на яку відгукуватиметься макрос, або Esc, щоб відкинути його.
macro-describe = Для чого макрос на { $key }
macro-saved = макрос на { $key } збережено, кроків: { $steps }
macro-none = макросів ще не записано · Ctrl+. записує макрос
macro-list-title = Макроси
macro-play-title = Як його відтворити
macro-play-once = Один раз, з місця, де стоїть курсор
macro-play-each = По разу для кожного позначеного файлу
macro-sure-title = Відтворити цей макрос
macro-sure-body = Макрос на { $key } копіює, переміщує, видаляє або перезаписує. Відтворити його?
macro-too-deep = макрос не може відтворювати сам себе нескінченно
plugin-list-title = Команди плагінів
plugin-none-title = Плагінів не встановлено
plugin-none-open = Відкрити папку плагінів ({ $dir })
plugin-none-help = Як працюють плагіни і де їх знайти
plugin-help-heading = Плагіни
plugin-why-off = вимкнено в Параметри → Параметри плагінів
plugin-why-panel = лише в панелях
plugin-why-viewer = лише в програмі перегляду
plugin-why-editor = лише в редакторі
plugin-why-local = потрібна папка на дисках цього комп'ютера
plugin-why-files = потрібен файл під курсором або позначений
plugin-why-masks = лише для { $masks }
plugin-why-bad-mask = його маска неправильна: { $error }
plugin-why-here = потрібні { $names } у цій папці
plugin-why-inside = потрібні { $names } тут або в папці вище
plugin-cannot = { $label }: { $why }
plugin-running = { $label }…
plugin-on-selected = для позначених файлів: { $count }
plugin-failed = { $id }: { $why }
plugin-unknown-command = плагін запитав `{ $id }`, а такої команди немає
hex-copied = { $size } скопійовано з { $from }
hex-moved = { $size } переміщено з { $from }
menu-calculator = Калькулятор
menu-terminal = Термінал
calc-title = Калькулятор
calc-decimal = десяткове
calc-hex = шістнадцяткове
calc-octal = вісімкове
calc-binary = двійкове
calc-characters = символи

# The archive commands (Shift+F3) and the two parts of the order that are not the key.
arc-commands-title = Архівні команди
arc-password = Пароль для { $name }
arc-op-test = Перевірити { $name }
arc-op-extract-here = Розпакувати { $name } тут
arc-op-extract-to = Розпакувати { $name } в…
arc-op-delete = Видалити { $name }
arc-op-add = Додати файли до архіву…
job-test = Перевірка архіву
msg-dirs-first-on = папки спочатку
msg-dirs-first-off = папки разом із файлами
msg-selected-first-on = позначені файли спочатку
msg-selected-first-off = позначені файли за порядком

# The change-drive dialog (Alt+F1, Alt+F2): the type column, and its footer.
drive-fixed = Диск
drive-removable = Змінний
drive-optical = Оптичний
drive-network = Мережевий
drive-memory = Пам'ять
drive-home = Домівка
drive-connection = З'єднання
drive-objects = Сховище об'єктів
drive-netbox = NetBox: збережені сеанси
drive-processes = Список процесів
drive-temp = Тимчасова панель
temp-added = додано до тимчасової панелі: { $count }
proc-none = список запущених процесів тут не читається
drive-programs = Встановлені програми
drive-services = Служби
svc-none = на цьому комп'ютері немає диспетчера служб, якого можна запитати
temp-not-a-file = рядки тут не є файлами: видаляти нічого
svc-card = Служба
svc-card-id = Ім'я
svc-card-name = Показується як
svc-card-state = Стан
svc-card-description = Опис
svc-card-pid = Процес
svc-state-running = працює
svc-state-stopped = зупинено
svc-state-failed = збій
svc-state-other = інше
key-service-start = Старт
key-service-restart = Перезап
key-service-stop = Стоп
svc-ask-stop = Зупинити { $name }?
svc-ask-restart = Перезапустити { $name }?
btn-service-stop = Зупинити
btn-service-restart = Перезапустити
svc-working = { $name }: запит до системи…
svc-started = { $name } запущено
svc-stopped = { $name } зупинено
svc-restarted = { $name } перезапущено
svc-needs-root = { $name }: для цього потрібен root — програма не проситиме його від вашого імені
svc-failed = { $name }: { $error }
prog-none = тут ніщо не каже, що встановлено
prog-card = Встановлена програма
prog-no-uninstall = ніщо не каже, як видалити { $name }
prog-uninstall-ready = команда, що видаляє { $name }, стоїть у командному рядку, не запущена
prog-card-name = Ім'я
prog-card-version = Версія
prog-card-publisher = Видавець
prog-card-size = Розмір на диску
prog-card-installed = Встановлено
prog-card-location = Папка
prog-card-uninstall = Видаляється командою
drive-sockets = Список сокетів
sock-none = тут ніщо не слухає, або таблиця не читається
sock-no-process = ніщо не каже, який процес тримає цей сокет
proc-card = Процес
proc-gone = { $name } більше не працює
proc-card-name = Ім'я
proc-card-pid = Ідентифікатор процесу
proc-card-parent = Запущено процесом
proc-card-user = Користувач
proc-card-state = Стан
proc-card-threads = Потоки
proc-card-memory = Пам'ять
proc-card-mapped = Відображено
proc-card-bits = Розрядність
proc-card-started = Запущено
proc-card-exe = Виконуваний файл
proc-card-cwd = Папка
proc-card-command = Командний рядок
proc-card-group = Група
proc-card-sid = SID
proc-card-priority = Пріоритет
proc-card-session = Сеанс
proc-card-uptime = Працює вже
proc-card-cpu-user = Час процесора, власний
proc-card-cpu-kernel = Час процесора, у ядрі
proc-card-handles = Відкриті дескриптори
proc-card-faults = Помилки сторінок
proc-card-private = Приватна пам'ять
proc-card-swap = Вивантажено
proc-card-io-read = Прочитано
proc-card-io-write = Записано
proc-card-memory-peak = Пік робочого набору
proc-card-private-resident = Робочий набір, приватний
proc-card-swap-peak = Пік файлу підкачки
proc-card-virtual = Віртуальні байти
proc-card-virtual-peak = Пік віртуальних байтів
proc-card-paged-pool = Вивантажуваний пул
proc-card-nonpaged-pool = Невивантажуваний пул
proc-card-io-other = Інше введення-виведення
proc-card-io-read-ops = Операції читання
proc-card-io-write-ops = Операції запису
proc-card-io-other-ops = Інші операції
proc-kill-title = Завершення процесу
proc-kill-ask = Завершити { $name } [{ $pid }]? Усе, що він не зберіг, буде втрачено.
proc-killing = Завершення { $name }…
proc-killed = { $name } завершено
proc-kill-failed = { $name } не вдалося завершити: { $why }
proc-card-env = Середовище
proc-card-modules = Завантажені модулі
proc-card-version = Версія
proc-card-description = Опис
drives-hint = ↑↓ вибір · 0-9 панелі · Ctrl+1..9 колонки · F9 усі · Ctrl+R перечитати
drives-title = Змінити диск: що показувати
drives-col-type = тип
drives-col-device = пристрій
drives-col-label = мітку
drives-col-fs = файлову систему
drives-col-size = розміри
drives-col-removable = змінні диски
drives-col-panels = панелі, що не є дисками
drives-col-optical = оптичні диски
drives-col-network = мережеві ресурси
drives-shown = показується: { $what }
drives-hidden = приховується: { $what }
drives-reread = диски перечитано
bookmarks-hint = Enter перейти · Ins зберегти · Del прибрати · F4 назвати · Esc закрити

# The search dialog (F7), which asks the whole question in one place.
search-title = Пошук
search-for = Шукати
search-mode = Як
search-mode-text = текст
search-mode-hex = hex
search-mode-regex = регулярний вираз
search-case = Враховувати регістр
search-words = Лише цілі слова
btn-find-next = Знайти далі
btn-find-prev = Знайти назад
# The help: the chapters' own names, and the note on a chapter nobody has translated yet
help-chapter-about = Що це таке
help-chapter-panels = Панелі
help-chapter-files = Файли: копіювання, переміщення, пошук, архіви
help-chapter-viewer = Програма перегляду
help-chapter-editor = Редактор, шістнадцяткова панель і асемблер
help-chapter-network = Сервери
help-chapter-config = Конфігурація, клавіші та кольори
help-chapter-keys = Клавіші, як їх призначено зараз
help-chapter-commands = Усі команди та їхні клавіші
help-chapter-options = Усі параметри та їхні значення
help-here-panels = Панелі
help-here-viewer = Програма перегляду
help-here-editor = Редактор
help-here-hex = Шістнадцятковий редактор
help-here-dialog = Цей діалог
help-in-english = Цей розділ ще не перекладено вашою мовою, тому далі він англійською. Клавіші та параметри, які він називає, однакові в усіх мовах.
help-about-byline = FinalCommander { $version } — автор: Matteo Baccan
key-ok = OK
key-cancel = Скасув
key-dialog-move = Рухати
key-tree = Дерево
key-config = Налашт
browse-local-only = файл вибирається з цього комп'ютера; шлях на сервері вводиться вручну, і Tab його доповнює
key-browse = Огляд
table-prompt-filter = Рядки, що містять
table-sorted = відсортовано за { $column }
table-sorted-back = відсортовано за { $column }, у зворотному порядку
table-unsorted = знову в порядку, у якому вони у файлі
table-filtered = лишилося рядків: { $rows }
table-unfiltered = знову всі рядки
table-cut-sorted = з перших { $rows } рядків — це все, що було прочитано
table-too-many = забагато записів, щоб сортувати чи фільтрувати тут
table-copied =
    { $rows ->
        [one] { $rows } рядок у буфері обміну
        [few] { $rows } рядки в буфері обміну
       *[other] { $rows } рядків у буфері обміну
    }
table-headed = перший рядок — заголовок
table-not-headed = перший рядок — дані: колонки називаються за номерами
table-named-fields = цей формат сам називає свої поля: заголовка вгадувати не треба

# Splitting and combining (SPEC 03 §9)
split-title = Розрізання файлу
split-target = Куди
split-size = Кожна частина
split-size-hint = 1440K, 700M, 4G — остання частина містить те, що лишилося
split-count = Або на
split-count-hint = частин — число тут важливіше за розмір
split-bad-count = { $count } не є кількістю частин: дві або більше
split-local-file = Розрізання і збирання працюють із файлом на цьому комп'ютері
split-bad-size = «{ $size }» не є розміром: пишіть як 700M або 4G
combine-not-a-piece = Це не частина: частини розрізаного файлу закінчуються на .001, .002, … або мають поруч .crc
pieces-working = Розрізання або збирання: { $done } з { $total }
pieces-cancelled = Зупинено, і записане прибрано
split-done = записано частин: { $parts }, і { $list } поруч із ними
combine-done-checked = { $name } зібрано з частин: { $parts }, { $bytes }, і він відповідає своєму .crc
combine-done-unchecked = { $name } зібрано з частин: { $parts }, { $bytes } — файлу .crc для перевірки не було
pieces-failed = Не виконано: { $error }

# The branch view (SPEC 02 §3d)
branch-reading = Читання всіх файлів у { $path }…
branch-title = Гілка: { $path }
branch-done = файлів: { $count }, з усіх папок нижче цієї · Ctrl+Alt+B повертає назад
branch-truncated = файлів: { $count }, і на цьому обхід зупинився: гілка більша, ніж має сенс показувати списком
branch-not-here = Вигляд гілки стосується папки, а ця панель — список

## The release check (SPEC 11 §5).
menu-check-update = Перевірити оновленн&я
opt-update-check = Перевіряти нові випуски раз на день
btn-open-page = Відкрити сторінку
update-checking = Запит до GitHub про останній випуск…
update-title = Оновлення
update-available = Вийшов FinalCommander { $version } (це { $current }).
update-available-status = Вийшов FinalCommander { $version }: Параметри → Перевірити оновлення
update-current = Це останній випуск, { $version }.
update-failed = Не вдалося запитати про останній випуск: { $why }

## Duplicate files (SPEC 06 §13).
menu-dupes = Пошук файлів-дублікатів
dupes-title = Файли-дублікати
dupes-recursive = Шукати в підпапках
dupes-min-size = Найменший файл
dupes-min-size-hint = Розмір, як його пишуть фільтри: 1, 100K, 5M. Порожні файли ніколи не порівнюються.
dupes-select = Позначити всі копії, крім найстарішої
btn-search = Шукати
dupes-not-here = Пошук дублікатів починається з папки або з позначеного.
dupes-bad-size = Не розмір: { $size }
dupes-looking = Пошук файлів-дублікатів…
dupes-progress = Дублікати: прохід { $pass } з 3, { $done }/{ $total }
dupes-none = Серед файлів ({ $count }) дублікатів немає.
dupes-panel = Дублікати: груп — { $groups }
dupes-done = груп дублікатів: { $groups }, у копіях { $size }.
dupes-truncated = Зупинено достроково: груп поки що { $groups }, у копіях { $size }.

## SQLite queries (SPEC 04 §4b).
sql-empty = Напишіть запит після sql:, наприклад sql: SELECT * FROM t
sql-no-database = sql: потребує бази даних SQLite: зайдіть у неї або поставте на неї курсор.
sql-running = Виконання запиту…
sql-failed = Запит не вдався: { $why }

## Asking a language model (SPEC 06 §14).
ai-none = Службу ШІ не налаштовано: додайте блок [[ai]] у fcmd.toml (name, url, model, api, token_env).
ai-title = Запитати ШІ
ai-no-thread = немає відповіді, яку можна продовжити: спершу запитайте через `ai:` або поставте курсор на відповідь у папці відповідей
ai-nothing-to-explain = тут нема чого пояснювати: відкрийте файл у програмі перегляду чи редакторі або запитайте одразу після команди, що завершилась помилкою
ai-explain-question = Що це таке, з { $name }? Поясни коротко.
ai-tokens = токенів: { $in } на вході, { $out } на виході
ai-none-panel = Служб ШІ ще немає — F7 додає
ai-service-removed = { $name } видалено
ai-probing = Вхід до { $name } на { $host }…
ai-probe-ok = { $host } відповів HTTP { $status } за { $ms } мс, моделей: { $models } — введіть запитання і натисніть Enter
ai-probe-no-model = (моделі { $model } серед них немає)
ai-row-context = контекст
ai-row-conversation = розмова
ai-session-title = ШІ: { $name } · { $model } — файлів у контексті: { $files }
ai-context-added = { $count ->
        [one] { $count } файл
        [few] { $count } файли
       *[other] { $count } файлів
    } у контексті
ai-context-nothing = Нічого додавати: у контекст потрапляють лише файли з цього диска
ai-command-ready = команда стоїть у рядку — прочитайте її, перш ніж натискати Enter
ai-commit-question = Напиши повідомлення коміту для підготовлених змін.
ai-staged-diff = підготовлений diff
ai-send-diff = Підготовлений diff надсилається на { $host }
ai-send-files = Файлів: { $count }, { $size } — буде надіслано на { $host } разом із запитанням.
ai-asking = Запит до { $name }…
ai-busy = { $name } думає… { $secs } с · Esc зупиняє
ai-conversation-new = Нова розмова: її почне наступне запитання.
ai-live-empty = Введіть запитання в командному рядку: воно піде до { $name }, а розмова з'являтиметься тут.
ai-failed = ШІ не відповів: { $why }
ai-stopped = зупинено: запитання до { $name } відкинуто, і нічого не записано
ai-context-title = Файли в контексті
ai-context-all = Усі
ai-context-dropped = { $count ->
        [one] { $count } файл прибрано з контексту
        [few] { $count } файли прибрано з контексту
       *[other] { $count } файлів прибрано з контексту
    }
ai-context-hint = Enter або Del: прибрати з контексту — файл лишається на диску · F3: що надсилається · Esc
ai-threads-title = Розмови
ai-threads-hint = Enter: продовжити цю розмову · Esc
ai-thread-resumed = продовження { $name }
ai-next-estimate = далі ≈ { $tokens }
ai-next-over = далі ≈ { $tokens }, більше, ніж уміщує модель
ai-send-estimate = Загалом близько { $tokens } токенів, разом із попередньою розмовою.
ai-send-estimate-of = Загалом близько { $tokens } токенів, разом із попередньою розмовою, з { $max }, які вміщує модель.
ai-send-over = Загалом близько { $tokens } токенів: більше за { $max }, які вміщує модель, тож початок може бути обрізано. F8 прибирає файл.
ai-send-question = Запитання надсилається на { $host }.
btn-send = Надіслати

## Git (SPEC 06 §15).
menu-git = Git
git-title = Git
git-status = Що змінилося
git-log = Журнал
git-blame = Blame { $name }
git-branches = Гілки
git-local-only = git працює з папкою на цьому диску.
git-running = Запит до git…
git-failed = git: { $why }
git-clean = Нічого не змінилося.
git-status-title = Змінено: в індексі { $staged }, поза індексом { $unstaged }
git-side-conflict = конфлікт
git-side-staged = в індексі
git-side-changed = не в індексі
git-side-untracked = не відстежується
git-kind-modified = змінено
git-kind-added = новий
git-kind-deleted = видалено
git-kind-renamed = перейменовано
git-kind-copied = скопійовано
git-kind-type = тип змінено
git-kind-conflict = розв'язати
git-as-tree = Показати зміни деревом
git-as-list = Показати зміни списком
git-version-head = останній коміт
git-version-index = в індексі
git-version-disk = на диску
git-discard-title = Відкинути зміни
git-discard-body = Відкинути зміни { $count ->
        [one] { $count } файлу
       *[other] { $count } файлів
    }?
git-discard-worktree = назад до індексу:
git-discard-both = назад до останнього коміту:
git-discard-added = з індексу, лишається на диску:
git-discard-trash = до кошика:
git-discard-warning = Те, що повертається до індексу чи останнього коміту, скасувати не можна.
git-hint = Enter: вибрати · Esc: закрити

## The editor's character list and word completion.
charmap-title = Символи
charmap-hint = Друкуйте для звуження (символ або його код, U+00E9) · Enter: вставити · Esc: назад
editor-no-completion = Жодне інше слово у файлі так не починається.

## A stored session's address, copied (NetBox's Generate URL).
net-url-copied = Скопійовано { $url }

## The system log (SPEC 06 §16).
menu-syslog = Системний журнал
syslog-reading = Читання системного журналу…

## Deleting lines by pattern (FAR #10).
editor-prompt-delete-matching = Прибрати рядки, що відповідають (! на початку: залишити лише їх)
editor-lines-deleted = Прибрано рядків: { $count }

## The copy dialog's size policy (FAR #493).
copy-conflict-size = перезаписати, якщо розмір відрізняється

# Copy dialog: what an error does (FAR's issue #597)
copy-on-error = Якщо файл не вдається скопіювати
copy-error-ask = Запитати
copy-error-skip = Пропустити і продовжити
copy-error-stop = Зупинити копіювання

# Attributes dialog: the times copied from one file to another (FAR's issue #241)
attr-times-copied = Час скопійовано: Ctrl+Shift+V вставляє його в діалог іншого файлу
attr-times-none = Ще нічого не скопійовано: спершу Ctrl+Shift+C у діалозі файлу
attr-times-pasted = Час вставлено: OK його встановить

# Editor: replacements kept by name and recent ones (FAR's issue #282)
replace-tpl-title = Заміни
replace-tpl-hint = Enter виконує · Ins зберігає нещодавню під іменем · Del забуває збережену · Esc
replace-tpl-name = Зберегти цю заміну як
replace-tpl-kept = Збережено як {$name}
replace-tpl-empty = Замін ще немає: спершу Ctrl+F7

# Panel settings: a view and a sort per folder (FAR's issue #389)
opt-remember-view = Кожна папка має свій вигляд і сортування

# Session editor: a cap on a copy's speed to or from the site
site-max-kbps = Обмеження швидкості, КБ/с (порожньо: без обмеження)

# Session editor: IMAP, read only
site-protocol-imap = IMAP (пошта, лише читання)
site-protocol-imaps = IMAPS (пошта через TLS, лише читання)

# The AI services as a list, like the sessions
ai-services-title = Служби ШІ
ai-services-hint = Enter: до неї звертається ai: · Ins нова · F4 змінити · Del видалити · Esc
ai-service-title = Служба ШІ
ai-field-name = Ім'я
ai-field-api = Протокол
ai-field-url = Адреса
ai-field-model = Модель
ai-field-token-env = Ключ у змінній
ai-field-token-note = Порожньо: ключ запитується раз за сеанс і ніколи не записується.
ai-field-system = Що їй кажуть спочатку
ai-service-incomplete = Службі потрібні ім'я та адреса
ai-service-default = ai: тепер звертається до {$name}
ai-key-prompt = Ключ для {$name} (зберігається до виходу)
ai-api-openai = OpenAI і сумісні
ai-api-anthropic = Anthropic
ai-api-ollama = Ollama (на цьому комп'ютері)
menu-ai = Служби ШІ

# The secrets file
vault-password = Пароль файлу секретів fcmd
vault-new = Зберегти цей ключ у зашифрованому файлі секретів fcmd: виберіть його пароль (порожньо: лише на цей сеанс)
vault-refused = Файл секретів не відкрився: {$why}

# The programs from elsewhere
drive-tools = Зовнішні програми
drive-ai = ШІ
drive-git = Git
tools-found = знайдено
tools-missing = не встановлено
tools-summary = знайдено зовнішніх програм: { $found } з { $total }; F3 каже, для чого кожна, F4 встановлює відсутню
tools-card = Зовнішня програма
tools-card-program = Програма
tools-card-where = Де
tools-card-version = Версія
tools-card-for = Для чого
tools-card-install = Як встановити
tools-card-why = Чому fcmd запускає її, а не робить це сам: F1, «Сторонні програми».
tools-install-any = встановіть пакунок, що надає { $name }, і переконайтеся, що він є в PATH
tools-already = { $name } уже тут є
tools-for-archives = архіви: { $what }
tools-for-decompile = Shift+F6 у програмі перегляду для { $what }
tools-for-git = панель Git (Ctrl+Alt+G) і гілка в запрошенні
tools-for-video = кадри відео в програмі перегляду та швидкому перегляді (F3, Ctrl+Q)
tools-for-open = відкриття посилання в браузері
tools-for-syslog = Команди → Системний журнал
tools-for-services = панель служб
tools-card-site = Вебсайт
tools-no-package = Про жоден менеджер пакунків на цьому комп'ютері не відомо, що він має { $name }. Відкрити сторінку завантаження?
tools-install-ask = Встановити { $name }, виконавши в терміналі:
tools-install-how = Саме в терміналі sudo запитує ваш пароль. Ctrl+Alt+E повертає до панелей, а Ctrl+R у цій панелі перевіряє знову.
tools-install-else = Якщо не вдасться: { $url }
tools-install-failed = fcmd: { $name } не встановлено. Завантажте з { $url }
tools-installing = встановлення { $name } у терміналі: Ctrl+Alt+E повертає до панелей, Ctrl+R там перевіряє знову
btn-install = Встановити

# Plugins configuration
plugins-config-title = Плагіни
plugins-config-hint = Enter: увімкнути або вимкнути · F3 що це і що він дає · Esc
plugins-broken = не завантажився — F3 каже чому
plugins-cannot-toggle = плагін, що не завантажився, не можна ввімкнути: F3 каже, що з ним не так
plugins-on = { $id } увімкнено: його команди тепер у Ctrl+Shift+F11
plugins-on-restart = { $id } увімкнено: його команди тепер у Ctrl+Shift+F11, решта з'явиться з наступного запуску
plugins-off = { $id } вимкнено: його команди зникли з Ctrl+Shift+F11
plugins-off-restart = { $id } вимкнено: його команди зникли з Ctrl+Shift+F11, решта зникне з наступного запуску
plugins-state-on = увімк
plugins-state-off = вимк
plugins-card-id = Ідентифікатор
plugins-card-name = Ім'я
plugins-card-version = Версія
plugins-card-author = Автор
plugins-card-about = Опис
plugins-card-dir = Папка
plugins-card-state = Стан
plugins-card-brings = Дає
plugins-card-commands = Команди (Ctrl+Shift+F11):
plugins-card-notes = Сказано під час завантаження:
plugins-none = плагінів не знайдено · вони живуть у { $dir }, кожен у своїй папці з plugin.toml

# The git panel
git-file-log = Журнал { $name }
git-commit = Закомітити підготовлене…
git-pull = Pull
git-push = Push
git-pushing = Виконується push…
git-pulling = Виконується pull…
git-commit-prompt = Повідомлення коміту
git-commit-empty = Коміту потрібне повідомлення.
git-already-on = { $branch } — це гілка, на якій ви зараз.
git-switch-ask = Перейти на { $branch }? Файли, у яких гілки розходяться, зміняться на диску.
git-done = git: { $said }
git-branches-title = Гілки: { $count }
git-log-title = Журнал: { $count }
git-log-of-title = Журнал { $of }: { $count }
btn-switch = Перейти
key-git = Git
key-git-diff = Diff
key-git-stage = Індекс
key-git-unstage = ЗІндек
key-git-discard = Відкин
key-git-commit = Коміт
key-git-show = Показ
key-git-log = Журнал
key-git-changes = Зміни
key-git-pull = Pull
key-git-push = Push
key-git-branches = Гілки

undo-title = Скасування
btn-undo = Скасувати
menu-undo = Скасувати останню операцію
menu-strip-meta = Прибрати метадані зображень…
undo-nothing = нічого скасовувати: від запуску програми не було копіювання, переміщення, видалення до кошика чи нової папки
undo-copy =
    { $count ->
        [1] Надіслати до кошика файл, створений копіюванням:
        [one] Надіслати до кошика { $count } файл, створений копіюванням:
        [few] Надіслати до кошика { $count } файли, створені копіюванням:
       *[other] Надіслати до кошика { $count } файлів, створених копіюванням:
    }
undo-move =
    { $count ->
        [1] Перемістити файл назад, де він був:
        [one] Перемістити { $count } файл назад, де він був:
        [few] Перемістити { $count } файли назад, де вони були:
       *[other] Перемістити { $count } файлів назад, де вони були:
    }
undo-trash =
    { $count ->
        [1] Повернути файл із кошика:
        [one] Повернути { $count } файл із кошика:
        [few] Повернути { $count } файли з кошика:
       *[other] Повернути { $count } файлів із кошика:
    }
undo-mkdir = Видалити нову папку, якщо вона досі порожня:
strip-title = { $count ->
        [one] Метадані { $count } зображення
       *[other] Метадані { $count } зображень
    }
strip-all = Усе — текст, EXIF, XMP, IPTC, коментарі, C2PA
strip-ai = Лише те, що записала модель зображень — промпт, workflow, налаштування
strip-camera = Усе, крім EXIF камери та авторських прав — без GPS
strip-hint = Enter: прибрати; зображення не перекодовується, Ctrl+Alt+Z повертає назад · Esc
strip-nothing = жодного PNG, WebP чи JPEG на цьому диску не позначено і немає під курсором
strip-done = { $count ->
        [one] { $count } зображення
        [few] { $count } зображення
       *[other] { $count } зображень
    } без метаданих (прибрано блоків: { $blocks }), не мали чого прибирати: { $clean } · Ctrl+Alt+Z повертає їх
undo-restore = { $count ->
        [1] Повернути зображення з його метаданими:
        [one] Повернути { $count } зображення з метаданими:
        [few] Повернути { $count } зображення з метаданими:
       *[other] Повернути { $count } зображень із метаданими:
    }
undo-done =
    { $count ->
        [one] { $count } ім'я знову там, де було
        [few] { $count } імені знову там, де були
       *[other] { $count } імен знову там, де були
    }
slow-drives = список дисків зайняв { $secs } с
slow-drives-late = список дисків чекав { $secs } с: { $drives } не відповіли вчасно і показані без подробиць — мережевий диск, сервера якого немає?
slow-listing = читання { $what } зайняло { $secs } с
# editor search and undo (0.41)
btn-find-all = Усі збіги
btn-replace-all = Замінити все
btn-replace-one = Замінити наступний
editor-replace-title = Заміна
editor-replace-style = Зберігати стиль (foo → bar, Foo → Bar, FOO → BAR)
editor-replace-groups = У регулярному виразі $1 і { "${name}" } — те, що спіймала група, $$ — долар
editor-matches-title = Рядки з `{ $query }`: { $count }
editor-matches-hint = Друкуйте для звуження · Enter: перейти до рядка · Esc: назад


# sorting and extended attributes (0.41)
opt-sort-case = Великі й малі літери в сортуванні
opt-sort-case-auto = як у цій системі
opt-sort-case-yes = розрізняти
opt-sort-case-no = один алфавіт
copy-keep-xattrs = Зберігати розширені атрибути

# editor conversions, disk diff and guides (0.41)
editor-convert-title = Записати файл як
editor-convert-hint = Друкуйте для звуження · Enter: перетворити (F2 записує, Ctrl+Z повертає) · Esc: назад
editor-convert-eol = Кінець рядка: { $eol }
editor-convert-bom-on = Мітка порядку байтів (BOM): записується
editor-convert-bom-off = Мітка порядку байтів (BOM): не записується
editor-convert-page = Кодова сторінка: { $page }
editor-converted-eol = { $name }: під час наступного збереження рядки закінчуватимуться на { $what }
editor-converted-bom-on = { $name }: наступне збереження запише мітку порядку байтів
editor-converted-bom-off = { $name }: наступне збереження не запише мітки порядку байтів
editor-converted-page = { $name }: наступне збереження запише { $what }
editor-convert-same = Файл уже записано саме так
editor-convert-no-bom = { $page } не має мітки порядку байтів: вона є лише в UTF-8 і UTF-16
editor-convert-lossy = { $page } не вміщує всіх символів { $name }: нічого не змінено
btn-reload = Перечитати
btn-what-changed = Що змінилося
editor-diff-buffer = у редакторі
editor-diff-hint = Це порівняння лише показує, що змінилося — Esc повертає до запитання


# job journal and saved syncs (0.41)
journal-left = Копіювань, не завершених минулим сеансом: { $count } · Alt+Shift+F6 показує список
journal-title = Незавершені завдання
journal-hint = Enter продовжити · Del відкинути · Esc закрити
journal-none = Незавершених завдань не лишилося
journal-row = { $kind }: { $what } → { $target } · файлів готово: { $done }
journal-row-unreadable = не читається: { $file } ({ $error })
journal-unreadable = { $file }: це не журнал завдань, який уміє читати ця версія ({ $error }). Alt+Shift+F6 показує його, а Del там видаляє
journal-resumed = Продовження { $kind }: файлів уже було готово: { $done }
journal-discarded = Незавершене завдання відкинуто разом із тим, що воно записало наполовину
journal-cannot-resume = Цей журнал не читається; Del його видаляє
menu-jobs-left = Незавершені завдання
sync-report = Записати звіт про виконання в (файл)
sync-name = Зберегти цю пару як (ім'я)
sync-saved-button = Збережені…
sync-saved-title = Збережені синхронізації
sync-saved-hint = Enter виконати · F4 змінити · Del забути · Esc закрити
sync-saved-none = Жодної синхронізації ще не збережено: дайте їй ім'я в діалозі (Ctrl+Shift+Y)
sync-saved-removed = { $name } більше не зберігається
sync-report-local = Звіт має бути файлом на цьому комп'ютері
sync-report-written = Звіт про синхронізацію — у { $path }
sync-report-not-written = Звіт не вдалося записати в { $path }: { $error }
sync-report-title = Звіт про синхронізацію
sync-report-name = Ім'я: { $name }
sync-report-left = Ліва: { $path }
sync-report-right = Права: { $path }
sync-report-to-right = Скопійовано зліва → направо: { $count }
sync-report-to-left = Скопійовано справа → наліво: { $count }
sync-report-cut-right = Прибрано праворуч: { $count }
sync-report-cut-left = Прибрано ліворуч: { $count }
sync-report-skipped = Залишено як є, хоча відрізняються: { $count }
sync-report-failed = Не вдалося: { $count }
sync-report-stopped = Зупинено на півдорозі, тож могли бути виконані або ні: { $count }
sync-report-not-done = Не виконано, бо попередній крок не вдався або був зупинений: { $count }
menu-sync-saved = Збережені синхронізації

# hex editor search
hex-search-wrapped = `{ $query }`: знайдено, почавши знову з іншого кінця файлу
hex-replace-title = Заміна
hex-replace-with = Замінити на
btn-replace = Замінити
hex-replace-ask = Замінити { $was } у позиції { $at } на { $with }?
hex-replace-length = заміна має довжину { $with } байт, а збіг — { $was }: це зсунуло б усе після нього — спершу натисніть Ins для режиму вставки

# pixels and video (0.41)
opt-viewer-pixels = Зображення
opt-viewer-pixels-auto = справжні пікселі там, де термінал їх малює
opt-viewer-pixels-kitty = графічний протокол kitty
opt-viewer-pixels-iterm = вбудовані зображення iTerm2
opt-viewer-pixels-sixel = sixel
opt-viewer-pixels-off = завжди півблоки
video-needs-program = це відео ({ $what }): його кадри малюються через `{ $program }`, якої на цьому комп'ютері не встановлено — Shift+F11 однаково читає контейнер
video-no-frame = з цього відео немає кадру: { $why }
video-playing = відтворення зі швидкістю { $fps } кадрів за секунду, без звуку — Space ставить на паузу
video-paused = пауза на { $at } — Space продовжує, PgUp і PgDn крокують на десяту частину
video-ended = відео закінчилося — Space відтворює його знову


# finding things
menu-saved-searches = Збережені пошуки
drive-searches = Збережені пошуки
find-keys = F2 зберігає цей пошук під іменем · F9 збережені
find-save-name = Зберегти цей пошук як
find-saved = пошук збережено як { $name }
find-saved-none = жодного пошуку ще не збережено: F2 у діалозі пошуку зберігає
find-saved-title = Збережені пошуки — Enter відкриває, F4 редагує, Del забуває
find-saved-running = пошук { $name }…
find-saved-gone = пошук { $name } забуто
find-loaded = пошук { $name } завантажено
find-fuzzy = Приблизне ім'я: літери вище, по порядку, найкращий збіг першим
find-meta = Метадані (artist=bowie; camera=)
find-index = Запитати системний індекс замість обходу (може пропустити нові файли)
tools-for-index = індекс, який Alt+F7 може запитати замість обходу
menu-frecent = Часто відвідувані папки
frecent-none = жодної папки ще не відвідано
frecent-gone = { $path } прибрано з часто відвідуваних папок
frecent-hint = друкуйте літери шляху · Enter перейти туди · Del забути · Esc закрити

# ai third round
ai-redact-off = Маскування вимкнено: усе надсилається як є
ai-redact-none = Секретів для маскування не знайдено
ai-redact-count = Секретів замасковано перед надсиланням: { $count } ({ $kinds })
ai-redacted = секретів замасковано: { $count }
ai-preview-title = Що надсилається до { $name } ({ $host })
ai-preview-instruction = Інструкція
ai-preview-earlier = Попередня розмова
ai-preview-question = Запитання та його файли
ai-preview-key = F3 показує, що надсилається
ai-preview-reading = Читання того, що було б надіслано…
key-ai-redact = Маск.
ai-redact-now-on = Маскування ввімкнено: ключі, токени, приватні ключі та паролі маскуються, перш ніж щось надсилається
ai-redact-now-off = Маскування вимкнено: те, що надсилається, іде як є
ai-panel-redact-on = секрети замасковано
ai-panel-redact-off = секрети не замасковано
key-ai-local = Локал.
ai-row-refused = не цей комп'ютер: відхилено
ai-panel-local-only = лише локально
ai-local-now-on = Режим «лише локально» ввімкнено: служба, що не на цьому комп'ютері, відхиляється, і їй нічого не надсилається
ai-local-now-off = Режим «лише локально» вимкнено: можна звертатися до будь-якої служби зі списку
ai-local-refused = Режим «лише локально» ввімкнено: { $name } відповідає на { $host }, а це не цей комп'ютер — нічого не надіслано (F6 у панелі ШІ вимикає його)
ai-rename-prompt = Якими мають стати імена ({ $count })? (словами)
ai-send-names = Інструкція та { $count } з { $total } імен — лише імена, нічого з файлів — надсилаються на { $host }.
ai-rename-ready = шаблон моделі стоїть у діалозі: прочитайте попередній перегляд, виправте хибне, і лише OK перейменовує
ai-rename-waiting = шаблон перейменування від моделі готовий: Ctrl+Shift+F6 відкриває діалог із ним
ai-rename-unreadable = модель не відповіла шаблоном перейменування: її відповідь у папці відповідей (ai:), і нічого не перейменовано
ai-rewrite-prompt = Що зробити з виділеним блоком? (словами)
ai-rewrite-no-block = спершу виділіть блок (Shift+стрілки): переписування замінює саме виділене, а блок-колонка не має форми, у яку можна повернутися
ai-rewrite-read-only = цей файл тут лише для читання: переписаному не було б куди потрапити
ai-rewrite-before = блок
ai-rewrite-after = як його переписала модель
ai-rewrite-hint = Переписане моделлю проти блоку — Enter ставить його на місце блоку, Esc лишає блок як є
diff-hint-rewrite = Enter замінити блок · Esc залишити
ai-rewrite-applied = блок замінено переписаним моделлю — один Ctrl+Z повертає його
ai-rewrite-left = блок залишено як є
ai-rewrite-stale = блок уже не той, яким був, коли запитували переписування: нічого не замінено — версія моделі в папці відповідей (ai:)
ai-rewrite-same = модель повернула блок без змін: замінювати нічого
ai-answer-kept = відповідь — { $name } у папці відповідей (ai:) — редактор залишено попереду
ai-failure-question = Ця команда завершилась із кодом { $code }: `{ $command }` — чому вона не вдалася і що це виправить? Коротко.
ai-send-failure = Команда, що не вдалася, і останні рядки її виводу ({ $lines }) надсилаються на { $host }.

# archives and jobs
arc-test-whole = { $name }: перевірено записів: { $files }, { $bytes } — усі цілі
arc-test-bad-title = { $name }: { $bad } з { $files } записів виявилися пошкодженими

# extending it
plugin-column-failed = { $column }: { $error } — його колонка лишається порожньою для файлів, за які він не відповів
plugins-card-columns = Колонки (як їх називає [[column_set]]):
usermenu-far-row = Що прочитано інакше, ніж у FarMenu.ini ({ $count })
usermenu-far-notes = меню походить із FarMenu.ini від FAR: прочитано інакше речей: { $count } — останній рядок їх перелічує
usermenu-far-not-run = Нічого не запущено.
find-age = Змінено, днів тому (7, >=30, 7-30)
find-bad-age = `{ $text }` не є віком у днях — { $error }
find-label-age = { $days } д
find-keys-ai = Ctrl+Alt+Q описує пошук словами: мовна модель заповнює ці рядки
ai-find-prompt = Що ви шукаєте? (словами)
ai-send-words = Ці слова — і нічого з панелей: ні імені, ні шляху, ні файлу — надсилаються на { $host }.
ai-find-ready = пошук від моделі стоїть у діалозі: прочитайте рядки, виправте хибне, і лише OK шукає
ai-find-waiting = пошук від моделі готовий: Alt+F7 відкриває діалог із ним
ai-find-unreadable = модель не відповіла пошуком: її відповідь у папці відповідей (ai:), і нічого не шукалося
ai-find-left-out = { $said } — НЕ в рядках, бо вони не можуть цього висловити: { $what }
# treemap and compare
usage-map-title = Використання диска, усе дерево: { $path }
usage-map-walking = Вимірювання… файлів: { $files }, поки що { $size }
usage-map-where = { $path } · { $size } · файлів: { $files }
usage-map-unreadable = непрочитаних папок: { $count }
usage-map-rest = менших файлів: { $count }
usage-map-others = інших: { $count }
usage-map-empty = У цій папці ніщо не займає місця
usage-map-chosen = { $name } · { $size } · { $share }%
usage-map-hint = ←↑↓→ · Enter усередину · BkSp угору · Tab список · G панель туди · Esc
usage-map-hint-list = ↑↓ · Enter усередину · BkSp угору · Tab карта · G панель туди · Esc
usage-map-hint-walking = Esc зупиняє
usage-map-top = Це папка, яку вимірювали: вище за неї тут нічого немає
usage-map-no-file = Цей рядок — багато малих файлів, а не один, до якого може перейти панель
bin-comparing = Порівняння двох файлів байт за байтом… Esc зупиняє
bin-bytes = байтів: { $count }
bin-same = Обидва файли однакові, байт у байт: по { $bytes } байт
bin-where = відмінність { $at } з { $count } · перша на зсуві { $offset } · байтів, що відрізняються: { $differing }
bin-hint = Alt+↑↓ відмінності · ↑↓ PgUp PgDn Home End рядки · Esc закрити
diff-unified-on = Одна колонка: що пішло, потім що прийшло
diff-unified-off = Дві колонки, поруч
diff-patch-title = Зберегти відмінності як патч (.diff)
diff-patch-saved = { $name } записано: єдиний diff, рядків: { $lines }
diff-patch-nothing = Обидва файли однакові: патча для збереження немає
# wastebasket and external editor
drive-trash = Кошик
drive-trash-closed = Кошик (ця система не дає програмі його переглянути)
menu-trash = Кошик
menu-trash-empty = Очистити кошик
key-trash-restore = Віднов
key-trash-empty = Очист
btn-empty = Очистити
btn-rename = Перейменувати
trash-closed = кошик цієї системи можна відкрити лише з її власного файлового менеджера
trash-empty = кошик порожній
trash-failed = кошик: { $why }
trash-restored =
    { $count ->
        [one] { $count } елемент знову там, де був
        [few] { $count } елементи знову там, де були
       *[other] { $count } елементів знову там, де були
    }
trash-purged =
    { $count ->
        [one] { $count } елемент видалено назавжди
        [few] { $count } елементи видалено назавжди
       *[other] { $count } елементів видалено назавжди
    }
trash-card-name = Ім'я
trash-card-from = Був у
trash-card-deleted = Видалено
trash-card-size = Розмір
trash-clash-title = Відновлення
trash-clash-body = { $path } уже існує.
trash-clash-note = «Замінити» надсилає наявний до кошика; «Перейменувати» повертає цей поруч із ним.
trash-clash-more =
    { $count ->
        [one] Ще { $count } елемент у такому самому становищі.
        [few] Ще { $count } елементи в такому самому становищі.
       *[other] Ще { $count } елементів у такому самому становищі.
    }
trash-purge-title = Видалення назавжди
trash-purge-body =
    { $count ->
        [1] Видалити { $name } з кошика?
        [one] Видалити { $count } елемент із кошика?
        [few] Видалити { $count } елементи з кошика?
       *[other] Видалити { $count } елементів із кошика?
    }
trash-purge-note = Це видаляє остаточно: повернути їх нічим.
trash-empty-title = Очищення кошика
trash-empty-body = Видалити все, що є в кошику?
opt-editor-external = Зовнішній редактор
opt-editor-use-external = Використовувати зовнішній редактор для F4 (тоді Alt+F4 — вбудований)
opt-viewer-external = Зовнішня програма перегляду
opt-viewer-use-external = Використовувати зовнішню програму перегляду для F3 (тоді Alt+F3 — вбудована)


# editor features
opt-editor-line-numbers = Нумерувати рядки
editor-line-numbers-on = Номери рядків увімкнено
editor-line-numbers-off = Номери рядків вимкнено
editor-prompt-pipe-block = Пропустити блок через команду:
editor-prompt-pipe-file = Пропустити весь файл через команду:
editor-prompt-insert-output = Вставити те, що виводить ця команда:
editor-prompt-insert-file = Вставити файл:
editor-prompt-write-block = Записати блок у файл:
editor-pipe-running = Виконується { $command } …
editor-pipe-done = { $command }: замінено її виводом, рядків: { $lines } (Ctrl+Z повертає)
editor-pipe-inserted = { $command }: вставлено рядків: { $lines }
editor-pipe-empty = { $command } нічого не вивела: нічого не вставлено
editor-pipe-failed = { $command }: { $error } — нічого не змінено
editor-pipe-changed = { $command }: текст змінився, поки команда працювала — нічого не замінено
editor-pipe-gone = { $command }: файл, для якого її запускали, уже не відкрито — нічого не змінено
editor-pipe-column = Блок-колонку не можна пропустити через команду: виділіть цілі рядки або нічого — для всього файлу
editor-file-inserted = { $name } вставлено, рядків: { $lines }
editor-insert-not-local = З редактора можна прочитати або записати лише файл на цьому комп'ютері
editor-insert-too-big = завеликий, щоб прочитати в редактор
editor-block-written = Блок записано в { $name }: { $size }
editor-block-write-self = { $name } — це файл, який редагується: блок не записано поверх нього
editor-choice-hint = Друкуйте для звуження · Enter: вибрати · Esc: назад
editor-syntax-title = Розфарбувати файл як
editor-syntax-auto = Як каже його ім'я: { $language }
editor-syntax-auto-none = Як каже його ім'я: нічого відомого
editor-syntax-none = Звичайний текст, без кольорів
editor-syntax-set = { $name } тепер читається як { $language }
editor-syntax-off = { $name } тепер читається як звичайний текст
editor-case-heading = Змінити регістр
editor-case-upper = ВЕЛИКІ ЛІТЕРИ
editor-case-lower = малі літери
editor-case-title = Як У Заголовку: велика літера на початку кожного слова
editor-case-toggle = іНВЕРСІЯ: великі стають малими, а малі — великими
editor-no-word = Нічого не виділено, і курсор не стоїть на слові
editor-prompt-isearch = Пошук під час введення (↓ ↑ наступний і попередній, Esc повертає)
editor-word-only = Слово під курсором більше ніде не трапляється
editor-not-on-word = Курсор не стоїть на слові
editor-prompt-reflow = На скільки колонок зверстати абзац? (лише Enter: { $width })
editor-reflowed = Абзац зверстано на { $width } колонок (Ctrl+Z повертає)
editor-reflow-bad-width = { $width } не є шириною: кількість колонок від 8 до 1000
editor-no-paragraph = Курсор стоїть на порожньому рядку: абзацу тут немає
editor-no-name-yet = Цей файл ще не має імені: спершу збережіть його
editor-tools-title = Команди редактора
editor-tool-isearch = Пошук під час введення
editor-tool-word-next = Наступне місце, де трапляється слово під курсором
editor-tool-word-prev = Попереднє місце, де трапляється слово під курсором
editor-tool-pipe = Пропустити блок або весь файл через команду
editor-tool-write-block = Записати блок у файл
editor-tool-case = Змінити регістр блоку
editor-tool-reflow = Переверстати абзац на задану ширину
editor-tool-insert-file = Вставити файл у позицію курсора
editor-tool-insert-output = Вставити те, що виводить команда
editor-tool-insert-date = Вставити дату й час
editor-tool-insert-name = Вставити ім'я цього файлу
editor-tool-insert-path = Вставити повний шлях цього файлу
editor-tool-line-numbers = Номери рядків увімкнути або вимкнути
editor-tool-syntax = Вибрати мову, якою розфарбовується файл
