# Key bar labels: short, they must fit in a few columns.
key-help = Дапам
key-menu = КарМн
key-view = Прагл
key-edit = Змена
key-copy = Капія
key-print = Друк
key-move = Перан
key-mkdir = Катал
key-delete = Выдал
key-pulldown = КанфМн
key-quit = Выхад
# The key bar while a modifier is held: FAR's own words, six columns wide.
key-add = Дадаць
key-extract = Распак
key-arccmd = АрхКам
key-edit-new = Змена…
key-rename = Пераназ
key-group = Суполка
key-selup = Выбран
key-left = Левая
key-right = Правая
key-sort-name = Імя
key-sort-ext = Пашыр
key-sort-write = Запіс
key-sort-size = Памер
key-sort-unsort = Непарад
key-sort-create = Створ.
key-sort-access = Доступ
key-sort = Парадк
key-mklink = Спасыл
key-find = Пошук
key-history = Гістор
key-folders = ГістКат
key-plugins = Дадатак

# Panel chrome
list-up = Уверх
list-folder = Каталог
col-name = Імя
col-ext = Пашыр
col-size = Памер
col-allocated = Выдзелена
col-modified = Зменена
col-created = Створана
col-accessed = Доступ
col-attr = Атр
col-owner = Уладальнік
col-group = Суполка
col-descr = Апісанне
col-links = Спс
col-git = Git
descr-title = Апісанне { $name }
descr-set = { $name }: апісанне запісана
descr-cleared = { $name }: апісанне прыбрана
descr-not-here = апісанні захоўваюцца побач з каталогам на гэтым камп'ютары
col-date-time = Дата       Час
panel-list-count = элементаў у спісе: { $count }
panel-empty = (пуста)
panel-loading = чытанне…

# Prompts
prompt-select-mask = Адзначыць файлы па масцы:
prompt-deselect-mask = Зняць адзнаку па масцы:
prompt-goto = Перайсці ў каталог:
prompt-filter = Маска файлаў або частка імя:
prompt-search = Хуткі пошук:

# Dialogs
dialog-error = Памылка
dialog-help-hint = Esc вяртае да зместу · F7 шукае ва ўсіх раздзелах · F1 закрывае · ↑↓ PgUp PgDn гартаюць
dialog-help-index = FinalCommander { $version } — Дапамога
dialog-help-hint-index = Enter адкрывае раздзел · F7 шукае ва ўсіх раздзелах · Esc закрывае
help-search-prompt = Пошук у дапамозе
help-search-none = Ніводзін радок дапамогі не змяшчае «{ $query }»
help-search-title = «{ $query }» у дапамозе: радкоў — { $count }
output-search-prompt = Пошук у вывадзе
output-search-empty = Яшчэ нічога не выведзена
output-search-found = Радок { $line } з { $lines }
output-search-wrapped = Радок { $line } з { $lines }, зноў ад найноўшага
output-search-none = Ніводзін радок вываду не змяшчае «{ $query }»

# Status messages
msg-not-implemented = { $command } у гэтай зборцы яшчэ не рэалізавана
msg-selected = адзначана элементаў: { $count }
msg-deselected = знята адзнака з элементаў: { $count }
msg-mask-invalid = няправільная маска: { $error }
msg-theme = тэма: { $name }
msg-font-bigger = шрыфт: у тэрмінала запытаны большы
msg-font-smaller = шрыфт: у тэрмінала запытаны меншы
msg-font-reset = шрыфт: у тэрмінала запытаны пачатковы
msg-sort = парадкаванне: { $key }{ $reverse }
msg-sort-groups-on = суполкі парадкавання ўключаны ({ $count })
msg-sort-groups-off = суполкі парадкавання выключаны ({ $count })
msg-sort-groups-none = суполкі парадкавання не зададзены: гл. [[sort_group]] у канфігурацыі
msg-view = выгляд: { $mode }
msg-hidden-on = схаваныя файлы паказаны
msg-hidden-off = схаваныя файлы схаваны
msg-no-history = гісторыі больш няма
msg-tab-last = у панэлі заўсёды ёсць адна ўкладка
msg-tab-end = укладка ўжо на гэтым краі рада
tabs-title = Укладкі
msg-tab-moved = укладка перанесена на іншую панэль
msg-tab-locked = гэтая ўкладка замацавана за сваім каталогам
msg-tab-locked-on = укладка замацавана за сваім каталогам
msg-tab-locked-off = укладка адмацавана
msg-bookmark-set = спасылка { $digit }: { $path }
msg-bookmark-unset = спасылка { $digit } не зададзена
msg-bookmark-added = { $path } захавана
msg-bookmark-removed = { $path } прыбрана
msg-bookmark-known = гэты каталог ужо ёсць у спісе
msg-dirsize = { $path }: { $bytes }, файлаў: { $files }
msg-dirsize-many = вымярэнне каталогаў: { $count }…
usage-title = Выкарыстанне дыска: { $path } · { $size } · Tab: усё дрэва як карта
usage-not-here = выкарыстанне дыска вымяраецца для каталога на дыску, а не ў архіве ці спісе знойдзеных файлаў
msg-dirsize-no-folder = тут няма каталога для вымярэння

# Jobs: prompts, confirmations, progress, conflicts
prompt-copy-to = Капіяваць { $what } у:
prompt-move-to = Пераназваць або перанесці { $what } у:
prompt-mkdir = Стварыць каталог:
prompt-tab-name = Назва гэтай укладкі:
prompt-bookmark-name = Назва гэтага каталога:
what-selected = адзначаныя элементы ({ $count })
confirm-delete-title = Выдаленне
confirm-delete-body =
    { $count ->
        [1] Выдаліць { $name } ({ $bytes })?
        [one] Выдаліць { $count } элемент ({ $bytes })?
        [few] Выдаліць { $count } элементы ({ $bytes })?
       *[other] Выдаліць { $count } элементаў ({ $bytes })?
    }
prompt-link = Стварыць спасылку на { $name } як:
link-created = { $name } створана
confirm-wipe-title = Знішчэнне
confirm-wipe-body =
    { $count ->
        [1] Перазапісаць і выдаліць { $name } ({ $bytes })?
        [one] Перазапісаць і выдаліць { $count } элемент ({ $bytes })?
        [few] Перазапісаць і выдаліць { $count } элементы ({ $bytes })?
       *[other] Перазапісаць і выдаліць { $count } элементаў ({ $bytes })?
    }
confirm-wipe-honest = Байты перазапісваюцца адзін раз, пасля чаго файл выдаляецца. На SSD, у файлавай сістэме з капіяваннем пры запісе або са здымкамі старыя копіі могуць ацалець — а імя застаецца ў каталогу і журнале.
job-copy = Капіяванне
job-move = Перанос
job-delete = Выдаленне
job-wipe = Знішчэнне
job-trash = Перанос у сметнік
job-replace = Замена
job-pack = Запакоўванне
job-rebuild = Запіс архіва

# ── the attributes dialog (SPEC 10 §3.8) ───────────────────────────────────────────────
attr-title = Атрыбуты файла
attr-count = адзначана элементаў: { $count }
attr-now = зараз { $text } ({ $octal })
attr-owner-read = Уладальнік можа чытаць
attr-owner-write = Уладальнік можа запісваць
attr-owner-exec = Уладальнік можа выконваць
attr-group-read = Суполка можа чытаць
attr-group-write = Суполка можа запісваць
attr-group-exec = Суполка можа выконваць
attr-other-read = Усе могуць чытаць
attr-other-write = Усе могуць запісваць
attr-other-exec = Усе могуць выконваць
attr-setuid = Выконваць ад імя ўладальніка (setuid)
attr-setgid = Выконваць ад імя суполкі (setgid)
attr-sticky = Выдаляць можа толькі ўладальнік (sticky)
attr-readonly = Толькі для чытання
attr-hidden = Схаваны
attr-system = Сістэмны
attr-archive = Архіўны
attr-temporary = Часовы
attr-offline = Аўтаномны
attr-indexed = Індэксатар можа яго чытаць
attr-compressed = зціснуты
attr-encrypted = зашыфраваны
attr-sparse = разрэджаны
attr-reparse = кропка паўторнага аналізу
attr-also = таксама { $what }
attr-mtime = Зменена
attr-atime = Апошняе чытанне
attr-btime = Створана
attr-ctime = Запіс зменены
attr-btime-readonly = (час стварэння можна задаць толькі ў Windows)
attr-utc = Час у UTC
attr-owner = Уладальнік (карыстальнік:суполка)
attr-owner-is = уладальнік { $who }
btn-attr-original = Пачатковыя
btn-attr-current = Цяперашнія
btn-attr-blank = Ачысціць
attr-recursive = І ўсё, што ўнутры каталогаў
attr-bad-time = `{ $text }` не з'яўляецца датай і часам (YYYY-MM-DD HH:MM:SS)
attr-bad-owner = тут няма ўліковага запісу ці суполкі з імем `{ $text }`
attr-in-archive = гэтая змена перазапісвае архіў нанова, астатнія запісы застаюцца як ёсць
job-attrs = Усталяванне атрыбутаў

# ── the copy dialog (SPEC 03 §4) ───────────────────────────────────────────────────────
copy-title = Капіяванне
copy-title-move = Пераназыванне або перанос
copy-target = Куды
copy-conflict = Калі файл ужо існуе
copy-conflict-ask = спытаць
copy-conflict-overwrite = перазапісаць
copy-conflict-newer = перазапісаць, калі крыніца навейшая
copy-conflict-skip = прапусціць
copy-conflict-rename = даць копіі вольнае імя
copy-conflict-append = дапісаць у канец
copy-only-newer = Толькі файлы, навейшыя за наяўныя
copy-filter = Толькі імёны, што адпавядаюць масцы
copy-follow-links = Капіяваць тое, на што паказвае спасылка, а не спасылку
copy-keep-times = Захоўваць час
copy-keep-mode = Захоўваць правы доступу
copy-verify = Перачытаць кожны файл і параўнаць
copy-several = прызначэнняў: { $count }, адно за адным

# ── the terminal (SPEC 06 §4) ──────────────────────────────────────────────────────────
term-started = тэрмінал, запушчана { $shell }
term-hint = Ctrl+Alt+E: назад да панэляў
term-ended = ён завяршыўся — любая клавіша закрые гэта
job-mkdir = Стварэнне каталога
job-planning = планаванне…
job-counters = файлаў: { $files } / { $filestotal } · { $bytes } / { $bytestotal }
job-rates = { $speed }/с · прайшло { $elapsed } · засталося { $eta }
job-rates-no-eta = { $speed }/с · прайшло { $elapsed }
job-hint = Esc скасоўвае
job-hint-queue = Esc скасоўвае · B у фон
job-queued = { $kind } у чарзе, незавершаных заданняў перад ім: { $ahead } · Alt+Shift+F5 паказвае спіс
job-background = { $kind } працягваецца ў фоне · Alt+Shift+F5 паказвае заданні
job-asks = { $kind } чакае адказу · Alt+Shift+F5
job-paused = { $kind }: паўза
job-resumed = { $kind }: працягнута
job-retry-nothing = Няма чаго паўтараць
job-still-waiting = Гэтае заданне чакае сваёй чаргі
jobs-title = Заданні (незавершаных: { $active })
jobs-hint = Enter паказаць · Space паўза · Del спыніць · F5 паўтарыць · Ctrl+↑↓ парадак · Esc закрыць
jobs-quit-title = Заданні яшчэ выконваюцца
jobs-quit-body = Гэтыя заданні спыняцца, калі завяршыць праграму зараз:
job-state-running = выконваецца
job-state-paused = паўза
job-state-asking = пытаецца
job-state-queued = чакае
job-state-held = затрымана
job-state-done = гатова
job-state-failed = памылкі
job-state-cancelled = скасавана
job-done = { $kind }: файлаў: { $files }, { $bytes }
job-done-renamed = { $kind }: файлаў перанесена без капіявання: { $files }
job-done-skipped = , прапушчана: { $skipped }
job-done-verified = , праверана: { $verified }
job-done-unverified = , не ўдалося перачытаць тут: { $unverified }
job-done-errors = , памылак: { $errors }
job-cancelled = { $kind }: скасавана
job-errors-title = Памылкі
conflict-title = Прызначэнне ўжо існуе
conflict-dst = прызначэнне  { $path }   { $bytes }
conflict-src = крыніца      { $path }   { $bytes }
conflict-newer = крыніца навейшая
conflict-older = прызначэнне навейшае
btn-overwrite = Замяніць
btn-overwrite-all = Усе
btn-skip = Прапусціць
btn-skip-all = Прапусціць усе
btn-retry = Паўтарыць
btn-move = Перанесці
btn-delete = Выдаліць
btn-wipe = Знішчыць
btn-copy = Капіяваць
btn-discard = Адкінуць
btn-kill = Завяршыць
btn-print = Друк
btn-quit = Выйсці
btn-play = Прайграць
btn-run = Выканаць
btn-accept = Прыняць
btn-cancel = Адмена
msg-nothing-to-do = нічога не адзначана
cmd-complete-nothing = няма чым гэта дапоўніць
complete-several = імёнаў, што так пачынаюцца: { $count }
form-moving = стрэлкі рухаюць дыялог · Enter або Esc пакідае яго там
form-moved = дыялог застаецца там, дзе вы яго паставілі
msg-trash-refused = Сметнік гэтага не прыняў — Shift+Del выдаляе адразу.

# Checksums (SPEC 03 §9)
sum-title = Кантрольныя сумы
sum-algorithm = Хэш
sum-one-file = Адзін спіс для ўсіх
sum-name = Назва
sum-working = { $name }: { $done } з { $total }
sum-verifying = праверка па { $name }
sum-stopped = спынена
sum-written = { $name } запісана
sum-no-name = спісу патрэбна назва
sum-local-only = кантрольную суму можна падлічыць толькі для файла на гэтым камп'ютары
print-title = Друк
print-body = Надрукаваць {$what}?
print-sending = адпраўка на прынтар…
print-sent = адпраўлена на прынтар
print-failed = друк не ўдаўся ({$line}): {$error}
print-local-only = надрукаваць можна толькі файл на гэтай машыне
print-no-command = няма каманды друку: задайце [print] command або ўсталюйце lp ці lpr
sum-all-match = усе { $count } супадаюць
sum-verify-title = Кантрольныя сумы
sum-verify-summary = супадаюць: { $ok }, не супадаюць: { $bad }
sum-differs = не супадае
sum-missing = адсутнічае
sum-unreadable = не чытаецца

# Synchronising two directories (SPEC 03 §10)
sync-title = Сінхранізацыя
sync-between = { $left }  ↔  { $right }
sync-recursive = Таксама ўсё ў падкаталогах
sync-both-ways = У абодва бакі, перамагае навейшы
sync-delete-extra = Прапаноўваць прыбраць тое, чаго няма з іншага боку
sync-by-content = Параўноўваць байты, а не памер і час
sync-filter = Толькі імёны, што адпавядаюць масцы
sync-walking = прагляд { $at } — пакуль што { $pairs }
sync-planned = капіяваць: { $copies } ({ $bytes }), прыбраць: { $deletes }
sync-hint = Space наступны адказ · ← → кірунак · = пакінуць · F2 выканаць · Esc скасаваць
sync-hint-walking = Esc спыняе абход
sync-nothing-to-do = у кожным радку сказана пакінуць усё як ёсць
sync-local-only = сінхранізацыя можа чытаць і пісаць толькі на гэтым камп'ютары
sync-same-directory = абедзве панэлі ў адным каталогу

# What to run for a kind of file (SPEC 06 §5)
assoc-none = тут нішто не кажа, як адкрыць { $name }
usermenu-empty = Меню карыстальніка яшчэ няма: стварыце яго ў { $path }
usermenu-last-gone = радка, выбранага мінулым разам, у гэтым меню ўжо няма
usermenu-bad-condition = Умова незразумелая — { $problem }
assoc-desktop = Перадаць працоўнаму сталу
assoc-open-with = Адкрыць { $name } з дапамогай
assoc-no-pager = ні прызначэнне, ні $PAGER не кажуць, чым гэта чытаць
assoc-no-editor = ні прызначэнне, ні $VISUAL не кажуць, чым гэта рэдагаваць

# Viewer (F3)
key-wrap = Згарн
key-hex = Код
key-text = Тэкст
key-dump = Дамп
key-doc = Док
key-search = Пошук
key-code = Кадыр
key-follow = Сачыць
viewer-prompt-search = Шукаць ({ $mode }):
viewer-prompt-goto = Перайсці да зруху або NN% :
viewer-not-found = `{ $query }` не знойдзена
viewer-follow-on = сачэнне за канцом файла
viewer-follow-off = рэжым сачэння выключаны
viewer-codepage = кадыроўка: { $name }
viewer-wrap-mode = { $mode }
viewer-no-undo = папярэдняй пазіцыі няма
viewer-empty = (пусты файл)
viewer-not-markdown = гэта не файл Markdown, або ён завялікі для вёрсткі
viewer-not-table = гэта не таблічны файл, або ён завялікі, каб прачытаць яго цалкам
viewer-base64-done = Дэкадавана: { $bytes }
viewer-not-base64 = Гэты файл не з'яўляецца base64: { $why }
viewer-status = { $mode } { $codepage }  { $offset } / { $len }  { $percent }%

key-disasm = Дызасм
viewer-disasm-failed = не ўдаецца дызасэмбляваць { $name }: { $error }
viewer-decompile-running = { $tool } працуе над { $name }…
viewer-decompile-none = тут няма чым дэкампіляваць { $format }; усталюйце інструмент і дадайце правіла [[decompile]] у fcmd.toml
arc-needs-program = гэты архіў чытае `{ $program }`, якая на гэтым камп'ютары не ўсталявана
viewer-decompile-failed = { $tool }: { $error }
viewer-blame-derived = git blame датычыцца самога файла: спачатку вярніцеся да яго
viewer-derived = { $producer } для { $name }

# Editor (F4)
key-save = Захав
key-replace = Замена
editor-prompt-goto = Перайсці да радка або радок:слупок :
editor-prompt-save-as = Захаваць як:
editor-saved = { $name }: захавана, байтаў: { $bytes }
editor-nothing-to-save = { $name } не мае незахаваных змен
editor-not-found = `{ $query }` не знойдзена
editor-replaced = замен: { $count }
editor-read-only = { $name } толькі для чытання
editor-readonly-on = толькі для чытання: змены адхіляюцца
editor-readonly-off = рэжым «толькі для чытання» выключаны
editor-no-undo = больш няма чаго скасоўваць
editor-no-redo = няма чаго паўтараць
editor-discard-title = Незахаваныя змены
editor-discard-body = { $name } мае незахаваныя змены. Выйсці і страціць іх?
editor-changed-title = Зменена на дыску
editor-changed-body = { $name } зменены на дыску пасля адкрыцця. Захаваць паверх?
editor-reloaded = { $name } перачытаны з дыска
editor-failed = { $name }: { $error }

# netbox (SPEC 04 §6)
net-sites-title = Злучэнні
net-sites-hint = Enter злучыць · n URL · Ins захаваць · F4 змяніць · d раз'яднаць · Esc закрыць
net-no-sites = Сайтаў яшчэ няма. Націсніце `n`, каб увесці URL, або дадайце блокі [[site]] у канфігурацыю.
net-not-a-stored-session = гэты радок — адкрытае злучэнне, а не захаваны сеанс
net-site-forgotten = { $name } больш не захоўваецца
net-prompt-url = Злучыцца з:
net-prompt-password = Пароль для { $user }@{ $host }:
net-prompt-passphrase = Парольная фраза для ключа { $user }@{ $host }:
net-connecting = злучэнне з { $host }…
net-connected = злучана з { $host }
net-disconnected = { $host } раз'яднаны
net-not-connected = { $host } не злучаны
net-local-panel = гэтая панэль на лакальнай файлавай сістэме
net-nothing-pending = ніводнае злучэнне гэтага не чакае
net-hostkey-title = Невядомы ключ вузла
net-hostkey-body = { $host } няма ў known_hosts. Яго адбітак:
net-hostkey-question = Запісаць яго і злучыцца?
net-hostkey-changed-title = Ключ вузла ЗМЯНІЎСЯ
net-hostkey-changed-body = { $host } адказаў ключом, які не супадае з запісаным.
net-hostkey-changed-where = Запісаны — у { $path }, радок { $line }.
net-hostkey-changed-what = Пераўсталяваны сервер выглядае дакладна як нехта пасярэдзіне. Тут нічога не прымаецца: адкрыйце файл, прачытайце радок і прыбярыце яго самі, калі ведаеце, чаму ключ змяніўся.
btn-open-file = Адкрыць файл
net-fetching = атрыманне { $name }…
net-viewing-copy = { $name }: лакальная копія, атрыманая для прагляду
net-too-big = { $name } большы за { $limit }: павялічце net.fetch_limit_mb, каб атрымаць яго
net-editing = { $name }: рэдагуецца лакальная копія; захаванне запісвае яе назад
net-writing-back = запіс { $name } назад…
net-written-back = { $name }: запісана назад, байтаў: { $bytes }
net-write-back-failed = запісаць назад не ўдалося: { $error } — вашы змены ў { $local }
net-changed-title = Зменена на серверы
net-changed-body = { $name } зменены на серверы пасля атрымання. Запісаць паверх?
confirm-delete-permanent = Гэта выдаляе канчаткова: сметніка для гэтага няма.
confirm-trash-title = Перанос у сметнік
confirm-trash-body =
    { $count ->
        [1] Перанесці { $name } ({ $bytes }) у сметнік?
        [one] Перанесці { $count } элемент ({ $bytes }) у сметнік?
        [few] Перанесці { $count } элементы ({ $bytes }) у сметнік?
       *[other] Перанесці { $count } элементаў ({ $bytes }) у сметнік?
    }
confirm-trash-where = Яны трапляюць туды, дзе працоўны стол трымае выдаленыя файлы, і іх можна адтуль вярнуць.

# ---------------------------------------------------------------- menu bar (SPEC 10)
menu-left = &Левая
menu-right = &Правая
menu-files = &Файлы
menu-commands = &Каманды
menu-options = &Налады
menu-brief = &Кароткі
menu-medium = &Сярэдні
menu-full = &Поўны
menu-detailed = Па&драбязны
menu-owners = Ул&адальнікі файлаў
menu-links = Спасылкі файлаў
menu-alt-full = Поўны, памеры цалка&м
menu-descriptions = Апісанн&е
menu-long-descriptions = До&ўгія апісанні
menu-wide = &Шырокі
menu-info-panel = Панэл&ь даведкі
menu-quick-view = Хутк&і прагляд
menu-tree-panel = Д&рэва каталогаў
menu-tab-new = &Новая ўкладка
menu-tab-close = Закрыць укладку
menu-tab-next = Наступна&я ўкладка
menu-tab-prev = Папяр&эдняя ўкладка
menu-tab-move = Перанесці ўкладку на іншу&ю панэль
menu-tab-lock = Замацава&ць укладку за яе каталогам
menu-tab-name = Назваць гэтую ўкладку
menu-tab-list = Усе ўкладкі
menu-sort-modes = Стан&ы парадкавання
menu-hidden = Схаваныя і сістэмныя файлы
menu-filter = &Фільтр файлаў
menu-filter-sets = Найменныя фільтры
menu-panel-onoff = Панэль укл/&выкл
menu-reread = Пера&чытаць
menu-change-drive = &Змяніць дыск
menu-view = &Прагляд
menu-edit = &Змена
menu-edit-new = Змена новага &файла
menu-copy = &Капіяванне
menu-rename = П&еранос або пераназыванне
menu-link = Стварыць спасы&лку
menu-multirename = Пераназваць па &шаблоне…
menu-multirename-undo = Скасаваць апошняе групавое пераназыванне
menu-mkdir = &Стварэнне каталога
menu-delete = В&ыдаленне
menu-wipe = Зніш&чэнне
menu-checksum = Кантрольныя с&умы…
menu-checksum-verify = Праверыць па спісе
menu-split = Разрэзаць файл…
menu-combine = Сабраць файлы
menu-delete-permanent = Выдаліць канчаткова
menu-archive-add = З&апакаваць
menu-archive-extract = Распака&ваць
menu-archive-commands = Ка&манды пакоўшчыка
menu-archive-test = Праверыць архіў
menu-attributes = А&трыбуты файлаў
menu-apply-command = Ужыць каман&ду
menu-replace-in-files = Замена ў файлах…
menu-select-group = Адзначыць суп&олку
menu-select-ext = Адзначыць па гэтым пашырэнн&і
menu-unselect-ext = Зняць адзнаку па гэтым пашырэнні
menu-unselect-group = З&няць суполку
menu-invert-selection = Супрац&ьлеглая адзнака
menu-select-all = Адзначыць усё
menu-restore-selection = Узнавіць адзнаку
menu-dirsize = Памер катало&га
menu-find-file = &Пошук файла
menu-history = Г&історыя камандаў
menu-view-history = Гіс&торыя прагляду і змены
menu-folders-history = Гіст&орыя каталогаў
menu-usage = Выкарыстанне &дыска
menu-jobs = &Заданні
menu-branch = Плоскі выгляд галіны
menu-bookmarks = Спасылкі на каталогі
menu-swap-panels = Аб&мяняць панэлі
menu-sync-panel = &Сінхранізаваць іншую панэль
menu-panels-onoff = Панэлі ўкл/&выкл
menu-compare-folders = Пара&ўнанне каталогаў
menu-sync-folders = Сін&хранізацыя каталогаў
menu-compare-contents = Параўна&ць па змесце
menu-panelize = Вывад каманды на панэль
menu-view-filtered = Прагляд праз каманду
menu-learn-keys = Вывучыць клавішы
menu-tunnel = Ад&крыць тунэль…
menu-tunnels = Тун&элі…
tunnel-title = Тунэль: [лакальны:]вузел:порт
tunnel-list-title = Тунэлі
tunnel-none = ніводзін тунэль не адкрыты
tunnel-open = тунэль адкрыты: { $what }
tunnel-closed = тунэль закрыты
tunnel-failed = { $what }: { $why }
tunnel-needs-connection = тунэль ідзе праз злучэнне: спачатку адкрыйце яго праз Alt+N
menu-connections = Злучэнні
menu-disconnect = Раз'яднаць
menu-user-menu = Меню кар&ыстальніка
menu-associations = Прызначэнні ф&айлаў
menu-folder-shortcuts = Спасы&лкі на каталогі
menu-plugin-commands = Каманды дадаткаў
menu-screens-list = Спіс эк&ранаў
menu-macro-record = Запісаць макрас
menu-macro-list = Макрасы
menu-task-list = Спіс зада&ч
menu-system-settings = &Сістэмныя налады
menu-panel-settings = Налады па&нэлі
menu-interface-settings = Налады знешняга в&ыгляду
menu-languages = &Мовы
menu-themes = &Тэмы
menu-confirmations = &Пацверджанні
menu-viewer-settings = Налады пра&гляду
menu-editor-settings = Налады &рэдактара
menu-codepages = Ка&дыроўкі
menu-sort-groups = Суп&олкі парадкавання
menu-mask-groups = Маскі &файлаў
menu-settings-all = &Усе налады
menu-highlighting = Афарбоўка файла&ў
menu-plugins-config = Налады дадат&каў
menu-help = Дапамога
menu-save-setup = &Захаваць налады
menu-quit = Вы&хад
opt-natural-sort = Натуральны парадак лікаў
opt-allow-reverse = Дазволіць зваротнае парадкаванне
opt-select-folders = Маскі адзначаюць і каталогі
cmd-no-network-name = Шлях на гэтым дыску не мае сеткавага імя
opt-trash = Ужываць сістэмны Сметнік
opt-system-copy = Ужываць сістэмны метад капіявання
opt-save-history = Захоўваць гісторыю паміж сеансамі
opt-history-lifetime = Тэрмін захоўвання гісторыі, у днях
opt-forever = заўсёды
opt-fetch-limit = Абмежаванне памеру аддаленых файлаў
opt-keepalive = SSH keep-alive
opt-off = выключана
opt-show-hidden = Паказваць схаваныя і сістэмныя файлы
opt-column-titles = Паказваць назвы слупкоў
opt-status-line = Паказваць радок стану
opt-scrollbar = Паказваць паласу пракруткі
opt-free-space = Паказваць вольнае месца
opt-auto-refresh = Абнаўляць панэль аўтаматычна
opt-dirs-first = Каталогі спачатку
opt-selected-first = Адзначаныя файлы спачатку
opt-reverse = Зваротны парадак
opt-view = Выгляд панэлі
opt-sort = Парадкаваць па
opt-quick-search = Хуткі пошук
opt-quick-alt = з Alt
opt-quick-plain = па літарах
opt-key-bar = Паказваць радок клавішаў
opt-status-bar = Паказваць радок статусу
opt-syntax = Расфарбоўваць распазнаныя мовы
opt-markdown = Markdown, звярстаны для чытання
opt-table = Адкрываць .csv і .jsonl як табліцу
opt-keymap = Раскладка клавішаў
opt-clock = Гадзіннік у куце
opt-mouse = Падтрымка мышы
opt-clipboard = Дазволіць тэрміналу перадаваць буфер абмену (OSC 52)
opt-command-screen = Паказваць вывад каманды падчас яе працы
opt-menu-always = Заўсёды паказваць радок меню
opt-save-exit = Захоўваць налады пры выхадзе
opt-confirm-delete = Выдаленне
opt-confirm-overwrite = Перазапіс
opt-confirm-exit = Выхад
opt-confirm-editor = Выхад з рэдактара з незахаванымі зменамі
opt-applied = налады ўжыты
opt-saved = захавана ў { $path }
opt-set-title = { $key }
opt-set = { $key } = { $value }
opt-not-a-toggle = Гэта не налада «так ці не»: Enter дазваляе ўвесці значэнне
opt-settings-hint = ↑↓ выбар · друкуйце для звужэння · Enter або F4 значэнне · Space пераключыць · Del прадвызначанае · Esc закрыць
opt-language-set = мова: { $name }
sort-name = Імя
sort-ext = Пашырэнне
sort-size = Памер
sort-mtime = Час запісу
sort-btime = Час стварэння
sort-atime = Час доступу
sort-descr = Апісанне
sort-owner = Уладальнік
sort-git = Стан Git
key-sort-descr = Апісан
key-sort-owner = Уладал
sort-none = Не парадкаваць
sort-set = парадкаванне: { $name }
btn-ok = OK
pick-hint = ↑↓ выбар · друкуйце для фільтра · Enter ужыць · Esc закрыць
pick-empty = (няма з чаго выбіраць)
# ------------------------------------------------- the three histories (SPEC 06 §6)
hist-pinned = { $line } замацавана · цяпер нішто яго не выцесніць
hist-unpinned = { $line } больш не замацавана
hist-is-pinned = { $line } замацавана · Ins спачатку адмацоўвае
hist-clear-body = Ачысціць гэтую гісторыю ад усяго, што не замацавана?
hist-cleared = у гісторыі засталося толькі замацаванае
history-hint = ↑↓ выбар · Enter ужыць · Ins замацаваць · Shift+Del адзін · Del усе · Esc закрыць


# ------------------------------------------------- session editor (SPEC 10 §3.5)
site-new = Новае злучэнне
site-edit = Змена злучэння
site-name = Імя
site-host = Вузел
site-port = Порт
site-user = Карыстальнік
link-title = Стварэнне спасылкі
link-name = Спасылка
link-kind = Від
link-kind-relative = сімвалічная, адносная — перажывае перанос дрэва
link-kind-absolute = сімвалічная, абсалютная — перажывае перанос спасылкі
link-kind-hard = жорсткая — другое імя таго самага файла
menu-colors = Ко&леры…
palette-title = Колеры { $theme }
palette-hint = Enter колер · F2 захаваць · Esc закрыць
palette-ask = Колер для { $key }
palette-bad-color = { $value } не з'яўляецца колерам (назва, 0–255 або #rrggbb)
palette-saved = колеры запісаны ў тэму { $theme }
palette-unsaved = колеры зменены толькі на экране — F2 іх запісвае
opt-viewer-status-line = Паказваць верхні радок (імя і пазіцыя)
opt-viewer-scrollbar = Паказваць паласу пракруткі
opt-viewer-key-bar = Паказваць радок клавішаў
opt-viewer-wrap = Доўгія радкі
opt-viewer-wrap-off = не згортваць
opt-viewer-wrap-char = згортваць на краі
opt-viewer-wrap-word = згортваць па словах
opt-viewer-tab-width = Памер табуляцыі
opt-viewer-remember = Памятаць пазіцыю і кадыроўку кожнага файла
opt-editor-tabs = Табуляцыі
opt-editor-tabs-keep = пакідаць як ёсць
opt-editor-tabs-typed = разгортваць тое, што ўводзіцца
opt-editor-tabs-all = разгортваць усе пры адкрыцці файла
opt-editor-tab-width = Памер табуляцыі
opt-editor-auto-indent = Аўтаматычны водступ
opt-editor-whitespace = Паказваць прабелы і табуляцыі
opt-editor-cursor-after = Курсор становіцца пасля тэксту, знойдзенага пошукам
opt-editor-beyond-eol = Курсор можа выходзіць за канец радка
opt-editor-trim-trailing = Прыбіраць прабелы ў канцы радкоў пры захаванні
editor-not-a-character = гэтая клавіша не мае сімвала для ўстаўкі
paste-one-line =
    { $count ->
        [one] устаўлены толькі першы радок · яшчэ { $count } радок прапушчаны
        [few] устаўлены толькі першы радок · яшчэ { $count } радкі прапушчаны
       *[other] устаўлены толькі першы радок · яшчэ { $count } радкоў прапушчана
    }
clip-names =
    { $count ->
        [one] { $count } імя ў буферы абмену
        [few] { $count } імені ў буферы абмену
       *[other] { $count } імёнаў у буферы абмену
    }
clip-paths =
    { $count ->
        [one] { $count } шлях у буферы абмену
        [few] { $count } шляхі ў буферы абмену
       *[other] { $count } шляхоў у буферы абмену
    }
clip-line = камандны радок у буферы абмену
clip-selection = вылучанае — у буферы абмену
clip-off = буфер абмену выключаны — `ui.clipboard` у наладах
history-gone = { $name } ужо няма
hl-new = Новая суполка колераў
hl-edit = Суполка колераў
# ------------------------------------------------- the diff screen (SPEC 05 §3)
diff-title = Адрозненні
menu-diff = Паказаць адрозненні &файлаў
diff-needs-two = Пастаўце курсор на файл у кожнай панэлі
diff-not-here = { $name } не з'яўляецца файлам на гэтым дыску
diff-too-big = { $name } большы за { $most }
diff-same = Абодва файлы аднолькавыя
diff-where = блок { $at } з { $count } · радкоў: -{ $removed } +{ $added }
diff-no-next = Апошняе адрозненне
diff-no-previous = Першае адрозненне
diff-hint = Alt+↑↓ блокі · Tab бок · F5 узяць · F2 запісаць · U адзіны · Shift+F2 патч · W C E ігнараваць · Esc
diff-hint-git = Alt+↑↓ блокі · S дадаць блок у індэкс · Tab бок · F5 узяць · F2 запісаць · Esc закрыць
diff-hint-git-staged = Alt+↑↓ блокі · S прыбраць блок з індэкса · Tab бок · F5 узяць · F2 запісаць · Esc закрыць
diff-stage-edited = у параўнанні ёсць незапісаныя зліцці: спачатку F2, потым S
diff-merged-left = Левы файл узяў гэтае адрозненне
diff-merged-right = Правы файл узяў гэтае адрозненне
diff-no-hunk-here = Уверсе экрана няма адрознення — Alt+↓ падвядзе да яго
diff-nothing-to-write = Нічога не зліта
diff-written = запісана: { $count }
diff-unwritten = Зліццё не запісана — F2 запісвае, Esc яшчэ раз пакідае як ёсць
diff-ignore-space-on = Прабелы ігнаруюцца
diff-ignore-space-off = Прабелы зноў улічваюцца
diff-ignore-case-on = Рэгістр ігнаруецца
diff-ignore-case-off = Рэгістр зноў улічваецца
diff-ignore-eol-on = Канцы радкоў ігнаруюцца
diff-ignore-eol-off = Канцы радкоў зноў улічваюцца

grab-hint = ↑↓←→ рух · Shift+↑↓←→ адзначыць · Ctrl+A усё · Enter капіяваць · Esc закрыць
grab-copied = скапіявана радкоў экрана: { $rows }

# ------------------------------------------------- the code pages (SPEC 05 §1.12)
cp-title = Кадыроўкі
cp-name-title = Як назваць гэтую кадыроўку
cp-set = Чытаецца як { $name }
menu-panel-names = Кадыроўка імёнаў…
names-title = Чытаць імёны як
names-system = Як іх дае сістэма
names-set = Імёны чытаюцца як { $name }
names-system-set = Імёны такія, як іх дае сістэма
names-not-here = У гэтай сістэме імя не з'яўляецца байтамі, таму іншага спосабу яго прачытаць няма
cp-marked = { $name } цяпер сярод тых, якія перабірае F8
cp-unmarked = { $name } больш не сярод тых, якія перабірае F8
cp-hint = ↑↓ выбар · Enter чытаць так · Ins адзначыць для F8 · F4 пераназваць · Esc закрыць

# ------------------------------------------------- the named mask groups (SPEC 02 §5b)
mg-title = Суполка масак
mg-name = Імя
mg-mask = Маскі
mg-hint = Ins дадае суполку, Del прыбірае, F4 змяняе · `·` пазначае тры ўбудаваныя
mg-saved = <{ $name }> дзейнічае; «Захаваць налады» яе запіша
mg-removed = <{ $name }> прыбрана
mg-bad = <{ $name }>: { $problem }
mg-needs-name = імя без прабелаў, косак і вуглавых дужак
mg-needs-mask = суполка, якая нічога не азначае, нічому не адпавядае
mg-built-in = <{ $name }> — адна з трох убудаваных · F4 дае ёй іншыя маскі

sg-title = Суполка парадкавання
sg-name = Імя
sg-mask = Маскі
sg-place = Дзе
sg-place-front = перад астатнімі
sg-place-back = пасля астатніх
sg-hint = Ins дадае суполку, Del прыбірае, F4 змяняе, Ctrl+↑↓ перасоўвае
sg-saved = Суполка дзейнічае; «Захаваць налады» яе запіша
sg-removed = { $name } прыбрана
sg-bad = { $name }: { $problem }
sg-needs-mask = суполка без маскі нічога б не змяшчала
hl-name = Імя
hl-mask = Маска або маскі
hl-kind = Ужываецца да
hl-kind-both = файлаў і каталогаў
hl-kind-file = толькі файлаў
hl-kind-dir = толькі каталогаў
hl-size = Памер (1M-10M, -4K, 1G-)
hl-age = Узрост у днях (<=1, >=30, 7-30)
hl-date-field = Узрост лічыцца па
hl-attrs = Атрыбуты (h r x s b, - азначае «не павінна быць»)
hl-fg = Колер тэксту (#d7875f, red, file.archive)
hl-bg = Фон, запісваецца гэтак жа
hl-hint = ↑↓ выбар · Enter/F4 змяніць · Ins новая · Del прыбраць · Ctrl+↑↓ перасунуць · Esc закрыць
hl-saved = суполка дзейнічае · Налады → Захаваць налады запіша яе ў файл
hl-removed = { $name } прыбрана · Налады → Захаваць налады запіша гэта ў файл
hl-bad = { $name }: { $problem }
filter-off = Без фільтра: усе імёны
filter-hint = ↑↓ выбар · Enter ужыць · F4 змяніць · Ins новы · Del прыбраць · Esc закрыць
filter-new = Новы фільтр
filter-edit = Фільтр
filter-name-hint = з імем ён захоўваецца ў меню фільтраў; без імя ўжываецца адзін раз
filter-applied = фільтр уключаны: { $name }
filter-cleared = фільтр выключаны
filter-saved = { $name } дзейнічае на панэлі і ёсць у меню фільтраў · Налады → Захаваць налады запіша яго ў файл
filter-removed = { $name } прыбраны · Налады → Захаваць налады запіша гэта ў файл
editor-no-block = нічога не вылучана
editor-unique = прыбрана паўторных радкоў: { $count }
editor-no-bracket = тут няма дужкі, або ёй няма пары
editor-tabs-expanded = табуляцыі разгорнуты ў радках: { $lines } — захаванне гэта запіша
site-protocol = Пратакол
site-protocol-sftp = SFTP (праз SSH)
site-protocol-ftp = FTP
site-protocol-ftps = FTP з TLS
site-protocol-dav = WebDAV
site-protocol-davs = WebDAV праз HTTPS
site-auth = Аўтэнтыфікацыя
site-auth-agent = агент SSH
site-auth-key = файл ключа
site-auth-ask = спытаць пароль
site-key-file = Файл ключа
site-start-dir = Аддалены каталог
site-group = Суполка
site-keepalive = Keep-alive (с)
site-compress = Сцісканне
site-page-session = Сеанс
site-page-environment = Асяроддзе
site-page-directories = Каталогі
site-page-sftp = SFTP
site-password-note = Пароль не захоўваецца: ён будзе запытаны падчас злучэння.
site-env-eol = Пераўтвараць канцы радкоў (яшчэ не)
site-env-utf8 = Прымусова імёны ў UTF-8 (яшчэ не)
site-jump = Прамежкавы вузел
site-jump-note = Злучэнне ідзе праз гэты вузел: host, user@host або user@host:port. Пуста — адразу да сервера.
site-sftp-ciphers = Выбраць шыфры (яшчэ не)
btn-connect = Злучыць
site-needs-host = злучэнню патрэбен вузел
site-needs-key = выберыце файл ключа або іншы спосаб аўтэнтыфікацыі
site-bad-port = `{ $port }` не з'яўляецца лікам
site-stored = злучэнне захавана: { $name } (Shift+F9, каб запісаць)
# ------------------------------------------------- structure pane (SPEC 05 §1.8)
key-struct = Структ
key-markdown = MD
key-table = Табл
key-filter = Фільтр
key-decompile = Дэкамп
key-next = Далей
key-prev = Назад
key-codepages = Кадыр
key-goto = Перайсц
key-to-panel = Панэль
key-blame = Blame
viewer-structure = Структура
viewer-structure-hint = ↑↓ радок · Enter перайсці · Tab змест · Shift+F11 закрыць
viewer-structure-hint-tab = Tab пераводзіць клавішы сюды
viewer-no-structure = няма чаго дэкадаваць: панэль структуры не ведае гэтага фармату
viewer-no-target = гэты радок нікуды ў файле не паказвае
viewer-form-opened = Форма ў выглядзе тэксту, якім яе паказвае Delphi. F4 вяртае да байтаў.
viewer-metadata-opened = Увесь тэкст, звярстаны для чытання. F4 вяртае да байтаў.
viewer-went-to = пераход да { $offset }
viewer-in-panel = панэль стаіць на { $name }
viewer-not-in-panel = { $name } няма ў гэтай панэлі
viewer-no-more-files = у панэлі больш няма файлаў
editor-unsaved-for-viewer = спачатку захавайце: праграма прагляду чытае файл з дыска
# ------------------------------------------------- command line (SPEC 06 §7)
cmd-running = выконваецца: { $command }
prompt-panelize = Каманда, вывад якой называе файлы
prompt-viewer-filter = Каманда, вывад якой чытаць замест файла
learn-title = Якія клавішы пасылае гэты тэрмінал
learn-hint = Націскайце клавішы. Падсвечаная клетка азначае, што клавіша дайшла; Esc двойчы — выхад.
learn-last = апошняя: { $key } — { $command }
learn-unbound = ні да чаго не прывязана
recode-title = Перазапісаць як
recode-what = файлаў: { $count }
recode-from = Чым яны ёсць цяпер
recode-to = Запісаць іх як
recode-auto = вызначаць для кожнага файла
recode-eol = Канцы радкоў
recode-eol-keep = не чапаць
recode-working = перазапіс файлаў: { $count }
recode-done = запісана: { $changed }, ужо такія, як трэба: { $same }, адхілена: { $failed }
menu-recode = Перазапісаць як…
panelize-title = { $command }
panelize-done = файлаў: { $count }, прапушчана радкоў: { $dropped }
panelize-nothing = { $command }: ніводзін радок не назваў наяўнага файла
cmd-interrupted = камандзе паслана перапыненне
cmd-done = { $command } завершана
cmd-failed = { $command } завяршылася з кодам { $code }
cmd-busy = каманда ўжо выконваецца
shell-not-kept = { $shell } не пазначыў запрашэнне своечасова: кожная каманда цяпер выконваецца асобна, як раней
prompt-apply = Каманда для кожнага з файлаў ({ $count }):
cmd-apply-no-ask = каманду, якая пытаецца значэнне, нельга ўжыць да файлаў па адным
cmd-apply-progress = { $command }  ({ $done } з { $total })
cmd-apply-stopped = спынена пасля { $done } з { $total }: { $command } завяршылася памылкай
cmd-not-remote = камандзе патрэбен лакальны каталог; гэтая панэль на серверы
cmd-no-history = гісторыя камандаў пустая
cmd-no-previous = няма куды вяртацца
cmd-no-such-dir = { $path }: такога каталога няма
cmd-no-such-path = { $path }: такога файла ці каталога няма
cmd-no-selection = нічога не адзначана
cmd-condition-false = не выканана: { $command }
cmd-screen-cleared = экран ачышчаны
cmd-stack-empty = стэк каталогаў пусты
cmd-stack-cleared = стэк каталогаў ачышчаны
cmd-pushed = каталогаў у стэку: { $count }
cmd-environment = асяроддзе
cmd-set = { $name } = { $value }
cmd-not-set = { $name } не зададзена
user-screen-title = Вывад — радкоў: { $lines }, з радка { $at }
user-screen-empty = (яшчэ нічога не запускалася)
viewer-no-branch = у гэтым радку няма адраса для пераходу
viewer-no-back = няма куды вяртацца

# ------------------------------------------------- find files (SPEC 06 §1)
folders-title = Пошук каталога
folders-reading = Чытанне каталогаў…
folders-read = каталогаў: { $count }
folders-capped = каталогаў: { $count }, да мяжы
folders-local-only = спіс каталогаў датычыцца гэтага камп'ютара; сервер мае сваю гісторыю і спіс спасылак
find-title = Пошук файла
find-mask = Маска або маскі файлаў
find-size = Памер (100M, 1M-1G, -4K)
find-text = Змяшчае тэкст
find-case = Улічваць рэгістр
find-words = Толькі поўныя словы
find-hex = Тэкст вышэй — шаснаццатковы шаблон
find-not = Файлы, якія яго НЕ змяшчаюць
find-scope = Шукаць у
find-scope-panel = гэтым каталогу
find-scope-tree = гэтым каталогу і ніжэй
find-scope-selection = адзначаных каталогах
find-scope-root = ад кораня гэтага дыска
find-scope-other = іншай панэлі і ніжэй
find-links = Пераходзіць па сімвалічных спасылках
find-dirs = Каталогі таксама ў выніках
find-archives = Шукаць у архівах
find-results = Знойдзена { $count } · прагледжана файлаў: { $files }, каталогаў: { $dirs }
find-done = знойдзена { $count }, файлаў прагледжана: { $files } · { $elapsed }
find-none = нічога не знойдзена
find-stopped = спынена пасля прагляду файлаў: { $files }
find-hint = ↑↓ выбар · Enter перайсці · F3 прагляд · F4 змена · F5 на панэль · Esc спыніць
find-hint-done = ↑↓ выбар · Enter перайсці · F3 прагляд · F4 змена · F5 на панэль · Esc закрыць
find-panelised = вынікаў на панэлі: { $count }
find-needs-mask = патрэбна маска: `*` азначае ўсе файлы
find-bad-size = `{ $text }` не з'яўляецца памерам — { $error }
# ------------------------------------------------- replace across files
replace-title = Замена ў файлах
replace-needle = Што шукаць
replace-with = Замяніць на
replace-case = Улічваць рэгістр
replace-words = Толькі поўныя словы
replace-pattern = Тэкст вышэй — рэгулярны выраз
replace-hex = Абодва — шаснаццатковыя байты
replace-scope = адзначана файлаў: { $count }
replace-needs-needle = няма чаго шукаць
replace-confirm-title = Замена
replace-confirm-body = Будзе зменена ўваходжанняў: { $count }, у файлах: { $files }.
replace-nothing = няма чаго замяняць: ніводзін файл не зменіцца
replace-done = заменена ўваходжанняў: { $count }, у файлах: { $files }

# ── archives (SPEC 04 §4) ───────────────────────────────────────────────────────────────
arc-writing-back = Запіс { $name } назад у архіў
arc-move-out = каб забраць нешта з архіва, скапіюйце гэта праз F5
arc-add-files-only = па адным дадаюцца толькі файлы; Shift+F1 дадае цэлы каталог

rename-title = Групавое пераназыванне
rename-name = Імя
rename-ext = Пашырэнне
rename-search = Шукаць
rename-replace = Замяніць на
rename-regex = Рэгулярны выраз ($1 у замене)
rename-case = Літары
rename-case-same = як ёсць
rename-case-lower = малыя літары
rename-case-upper = ВЯЛІКІЯ ЛІТАРЫ
rename-case-title = Кожнае Слова З Вялікай
rename-case-sentence = Толькі першая літара вялікая
rename-counter = Лічыльнік: пачатак, крок, лічбы
rename-fields = [N] імя  [E] пашырэнне  [N1-3] частка  [C] лічыльнік  [d] [t] дата, час  [P] каталог
rename-not-in-archive = некалькі файлаў пераназываюцца ў каталогу, а не ў архіве: там F6 пераназывае адзін запіс
rename-bad-counter = лічыльнік — гэта тры цэлыя лікі: пачатак, крок, лічбы; { $counter } імі не з'яўляецца
rename-cannot = не ўдаецца пераназваць: { $why }
rename-problem-empty = імя не засталося
rename-problem-invalid = файл не можа мець такога імя
rename-problem-twice = гэтае імя дастаецца яшчэ аднаму файлу
rename-problem-exists = файл з такім імем ужо ёсць
rename-more = … і яшчэ { $count }
rename-blocked =
    { $count ->
        [one] { $count } файл не можа атрымаць новае імя: нічога не пераназываецца, пакуль гэта не выпраўлена
        [few] { $count } файлы не могуць атрымаць новыя імёны: нічога не пераназываецца, пакуль гэта не выпраўлена
       *[other] { $count } файлаў не могуць атрымаць новыя імёны: нічога не пераназываецца, пакуль гэта не выпраўлена
    }
rename-summary = змяняецца { $count } з { $total }
rename-nothing = ніводнае імя не змяняецца
rename-nothing-to-undo = няма групавога пераназывання, якое можна скасаваць
rename-done =
    { $count ->
        [one] пераназваны { $count } файл · скасаваць можна з меню Файлы
        [few] пераназваны { $count } файлы · скасаваць можна з меню Файлы
       *[other] пераназвана { $count } файлаў · скасаваць можна з меню Файлы
    }
rename-undone =
    { $count ->
        [one] вернута { $count } імя
        [few] вернута { $count } імені
       *[other] вернута { $count } імёнаў
    }
pack-title = Запакоўванне
pack-name = Архіў
pack-format = Фармат
pack-level = Сцісканне
pack-solid = Суцэльныя блокі (7z: меншы, але адзін файл чытаецца павольней)
pack-delete-after = Выдаліць файлы пасля запакоўвання
pack-level-normal = звычайнае
pack-level-fast = хуткае
pack-level-best = найлепшае
pack-scope = адзначана файлаў: { $count }
pack-needs-name = архіву патрэбна імя
pack-format-read-only = { $format } тут толькі чытаецца, а не запісваецца: выберыце іншы фармат або іншае імя
pack-single-one-file = { $format } змяшчае адзін файл: выберыце адзін або вазьміце { $tar } для каталога ці некалькіх файлаў
unpack-title = Распакоўванне архіва
unpack-to = Распакаваць у
unpack-scope = з { $name }
unpack-not-an-archive = гэта не архіў, які мы ўмеем чытаць

# ── the special panel kinds (SPEC 02 §7) ────────────────────────────────────────────────
panel-quick-view = Хуткі прагляд
panel-info = Даведка
panel-tree = Дрэва
compare-done = адрозніваюцца: { $left } злева, { $right } справа
cmp-subdirs = Заходзіць у падкаталогі
cmp-depth = Як глыбока заходзіць (0 = да самага нізу)
cmp-trees-title = Што адрозніваецца
cmp-trees-working = параўнанне дрэў: файлаў { $seen }, прачытана { $read }
cmp-trees-done = адрозніваюцца: { $differ }, толькі злева: { $left }, толькі справа: { $right }, усяго пар: { $pairs }
cmp-trees-same = абодва дрэвы аднолькавыя (пар: { $pairs }, праверана: { $same })
menu-duplicates = Пошук дублікатаў
dup-title = Файлы-дублікаты
dup-working = пошук дублікатаў: файлаў { $seen }, прачытана { $hashed }
dup-done = суполак копій: { $groups }, файлаў: { $files }, змарнавана { $wasted }
dup-none = тут нішто не з'яўляецца копіяй чагосьці іншага (файлаў: { $seen })
dup-stopped = спынена пасля прагляду файлаў: { $seen }
dup-partial = спынена датэрмінова
cmp-title = Параўнанне
cmp-case = Супастаўляць імёны з улікам рэгістра
cmp-size = Іншы памер — гэта адрозненне
cmp-time = Іншы час — гэта адрозненне
cmp-seconds = Час з розніцай да столькіх секунд лічыцца аднолькавым
cmp-contents = Чытаць байты
cmp-when-time = Чытаць іх толькі там, дзе час адрозніваецца
cmp-fragment = Чытаць толькі столькі першых КіБ (0 = усё)
cmp-ignore-eol = Адны толькі канцы радкоў не з'яўляюцца адрозненнем
cmp-selected = Толькі адзначанае
cmp-stop = Спыніцца на першым адрозненні
cmp-newer = Адзначаць толькі навейшую копію
menu-compare-setup = Параў&нанне з наладамі
compare-contents-eol-ignored = адрозніваюцца толькі канцамі радкоў, праігнаравана: { $count }
compare-contents-trusted = прызнана аднолькавымі без чытання: { $count }
compare-contents-working = Параўнанне зместу: { $done } з { $total }
compare-contents-done = адрозніваюцца пар: { $differ } з { $pairs }
compare-contents-same = усе пары ({ $pairs }) ідэнтычныя
compare-contents-none = ніводзін файл не мае аднолькавага імя з абодвух бакоў
compare-contents-local = для параўнання па змесце абедзве панэлі павінны быць на лакальнай файлавай сістэме
compare-contents-eol = толькі канцы радкоў: { $count }
compare-contents-only = толькі з аднаго боку: { $count }
compare-contents-unread = не ўдалося прачытаць: { $count }
compare-contents-stopped = параўнанне спынена
quick-view-directory = { $name } — каталог: F3 яго вымярае
quick-view-not-here = байтаў тут няма: F3 спачатку іх дастане
info-path = Шлях
info-kind = Від
info-kind-local = каталог на гэтым камп'ютары
info-kind-remote = на { $host }
info-kind-archive = унутры { $name }
info-kind-list = спіс файлаў
info-device = Прылада
info-folder = Каталог
info-device-value = { $total }, вольна { $free }, кластары { $cluster }
info-slack = Страты на кластарах
info-filesystem = Файлавая сістэма
info-mount = Змантавана
info-git = Рэпазіторый
info-git-clean = як у індэксе
info-git-differing = { $count } з { $tracked } адрозніваюцца ад індэкса
info-git-uncounted = не падлічана
info-git-merge = ідзе зліццё
info-git-rebase = ідзе rebase
info-git-cherry-pick = ідзе cherry-pick
info-git-revert = ідзе revert
info-git-bisect = ідзе bisect
info-memory = Памяць
info-memory-value = вольна { $free } з { $total }
info-memory-total = { $total }
info-contents = Змяшчае
info-contents-value = файлаў: { $files }, каталогаў: { $dirs }, { $bytes }
info-selected = Адзначана
info-selected-value = файлаў: { $count }, { $bytes }
info-cursor = Курсор
info-size = Памер
info-modified = Зменена
info-description = Апісанне
key-mark = Адзн
key-asm = Асм
key-block = Блок
key-screens = Экраны
key-site-edit = Змена
key-site-new = Новы
key-site-forget = Выдал
key-ai-edit = Змена
key-ai-new = Новы
key-ai-remove = Выдал
key-ai-forget = Забыць
key-ai-threads = Гістор
hex-needs-a-file = гэты буфер не мае файла, з якога чытаць байты
hex-save-first = спачатку захавайце: два рэдактары не могуць трымаць незахаваныя змены адзін аднаго

# ── the hex editor's block operations (SPEC 05 §2.6) ────────────────────────────────────
hex-block-title = Блок
hex-op-write = Запісаць { $size } у файл
hex-op-read = Прачытаць файл у пазіцыю { $at }
hex-op-insert = Уставіць нулявыя байты ў пазіцыю { $at }
hex-op-copy = Скапіяваць байты з іншага месца ў { $at }
hex-op-move = Перанесці байты з іншага месца ў { $at }
hex-op-fill = Запоўніць { $size } байтам
hex-op-xor = Ужыць выключальнае АБО з ключом да { $size }
hex-op-delete = Выдаліць { $size } — { $was } стане { $becomes }
hex-write-title = Запісаць блок у
hex-read-title = Які файл прачытаць
hex-fill-title = Якім байтам запоўніць (hex)
hex-xor-title = Выключальнае АБО з якім ключом (hex)
hex-insert-title = Колькі нулявых байтаў
hex-copy-title = Адкуль капіяваць — зрух і даўжыня (hex)
hex-move-title = Адкуль перанесці — зрух і даўжыня (hex)
hex-not-hex = гэта не паслядоўнасць шаснаццатковых лічбаў
hex-not-a-count = гэта не колькасць байтаў
hex-not-a-range = гэта не зрух і даўжыня ў hex
hex-range-outside = гэты дыяпазон па-за файлам ({ $size })
hex-move-into-itself = блок нельга перанесці ўнутр самога сябе
hex-filled = запоўнена { $size }
hex-xored = да { $size } ужыта выключальнае АБО — гэта не шыфраванне
hex-written = { $size } запісана ў { $name }
hex-read-done = { $size } прачытана з { $name }
hex-inserted = устаўлена нулёў: { $size }
hex-deleted = выдалена { $size }
hex-device = { $name }: прылада памерам { $size }, адкрыта толькі для чытання
asm-title = Асэмбляваць у { $at } ({ $arch })
asm-no-arch = { $arch } не з'яўляецца архітэктурай, для якой мы ўмеем асэмбляваць
asm-written = { $text } — { $bytes }
asm-written-longer = { $text } — { $bytes } (на { $grew } байт даўжэй)
screens-title = Экраны
screens-panels = Панэлі
quit-title = Выхад з FinalCommander
quit-body = Выйсці?
screens-quit-title = Незахаваныя змены
screens-quit-body = Гэтыя экраны змяшчаюць змены, якіх ніхто не запісаў:
macro-recording = Запіс макраса · Ctrl+. спыняе яго
macro-empty = нічога не запісана
macro-dropped = запіс адкінуты
macro-key-title = Якая клавіша
macro-key-body = Запісана крокаў: { $steps }. Націсніце клавішу, на якую будзе адгукацца макрас, або Esc, каб адкінуць яго.
macro-describe = Для чаго макрас на { $key }
macro-saved = макрас на { $key } захаваны, крокаў: { $steps }
macro-none = макрасы яшчэ не запісаны · Ctrl+. запісвае макрас
macro-list-title = Макрасы
macro-play-title = Як яго прайграць
macro-play-once = Адзін раз, з месца, дзе стаіць курсор
macro-play-each = Па разе для кожнага адзначанага файла
macro-sure-title = Прайграць гэты макрас
macro-sure-body = Макрас на { $key } капіюе, пераносіць, выдаляе або перазапісвае. Прайграць яго?
macro-too-deep = макрас не можа прайграваць сам сябе бясконца
plugin-list-title = Каманды дадаткаў
plugin-none-title = Дадаткі не ўсталяваны
plugin-none-open = Адкрыць каталог дадаткаў ({ $dir })
plugin-none-help = Як працуюць дадаткі і дзе іх знайсці
plugin-help-heading = Дадаткі
plugin-why-off = выключаны ў Налады → Налады дадаткаў
plugin-why-panel = толькі ў панэлях
plugin-why-viewer = толькі ў праграме прагляду
plugin-why-editor = толькі ў рэдактары
plugin-why-local = патрэбен каталог на дысках гэтага камп'ютара
plugin-why-files = патрэбен файл пад курсорам або адзначаны
plugin-why-masks = толькі для { $masks }
plugin-why-bad-mask = яго маска няправільная: { $error }
plugin-why-here = патрэбны { $names } у гэтым каталогу
plugin-why-inside = патрэбны { $names } тут або ў каталогу вышэй
plugin-cannot = { $label }: { $why }
plugin-running = { $label }…
plugin-on-selected = для адзначаных файлаў: { $count }
plugin-failed = { $id }: { $why }
plugin-unknown-command = дадатак запытаў `{ $id }`, а такой каманды няма
hex-copied = { $size } скапіявана з { $from }
hex-moved = { $size } перанесена з { $from }
menu-calculator = Калькулятар
menu-terminal = Тэрмінал
calc-title = Калькулятар
calc-decimal = дзесятковае
calc-hex = шаснаццатковае
calc-octal = васьмярковае
calc-binary = двайковае
calc-characters = сімвалы

# The archive commands (Shift+F3) and the two parts of the order that are not the key.
arc-commands-title = Каманды пакоўшчыка
arc-password = Пароль для { $name }
arc-op-test = Праверыць { $name }
arc-op-extract-here = Распакаваць { $name } тут
arc-op-extract-to = Распакаваць { $name } у…
arc-op-delete = Выдаліць { $name }
arc-op-add = Дадаць файлы ў архіў…
job-test = Праверка архіва
msg-dirs-first-on = каталогі спачатку
msg-dirs-first-off = каталогі разам з файламі
msg-selected-first-on = адзначаныя файлы спачатку
msg-selected-first-off = адзначаныя файлы па парадку

# The change-drive dialog (Alt+F1, Alt+F2): the type column, and its footer.
drive-fixed = Дыск
drive-removable = Зменны
drive-optical = Аптычны
drive-network = Сеткавы
drive-memory = Памяць
drive-home = Дамашні
drive-connection = Злучэнне
drive-objects = Сховішча аб'ектаў
drive-netbox = NetBox: захаваныя сеансы
drive-processes = Спіс працэсаў
drive-temp = Часовая панэль
temp-added = дададзена на часовую панэль: { $count }
proc-none = спіс запушчаных працэсаў тут не чытаецца
drive-programs = Усталяваныя праграмы
drive-services = Службы
svc-none = на гэтым камп'ютары няма дыспетчара службаў, якога можна спытаць
temp-not-a-file = радкі тут не з'яўляюцца файламі: выдаляць няма чаго
svc-card = Служба
svc-card-id = Імя
svc-card-name = Паказваецца як
svc-card-state = Стан
svc-card-description = Апісанне
svc-card-pid = Працэс
svc-state-running = працуе
svc-state-stopped = спынена
svc-state-failed = збой
svc-state-other = іншае
key-service-start = Старт
key-service-restart = Перазап
key-service-stop = Стоп
svc-ask-stop = Спыніць { $name }?
svc-ask-restart = Перазапусціць { $name }?
btn-service-stop = Спыніць
btn-service-restart = Перазапусціць
svc-working = { $name }: запыт да сістэмы…
svc-started = { $name } запушчана
svc-stopped = { $name } спынена
svc-restarted = { $name } перазапушчана
svc-needs-root = { $name }: для гэтага патрэбен root — праграма не будзе прасіць яго ад вашага імя
svc-failed = { $name }: { $error }
prog-none = тут нішто не кажа, што ўсталявана
prog-card = Усталяваная праграма
prog-no-uninstall = нішто не кажа, як выдаліць { $name }
prog-uninstall-ready = каманда, якая выдаляе { $name }, стаіць у камандным радку, не запушчаная
prog-card-name = Імя
prog-card-version = Версія
prog-card-publisher = Выдавец
prog-card-size = Памер на дыску
prog-card-installed = Усталявана
prog-card-location = Каталог
prog-card-uninstall = Выдаляецца камандай
drive-sockets = Спіс сокетаў
sock-none = тут нішто не слухае, або табліца не чытаецца
sock-no-process = нішто не кажа, які працэс трымае гэты сокет
proc-card = Працэс
proc-gone = { $name } больш не працуе
proc-card-name = Імя
proc-card-pid = Ідэнтыфікатар працэсу
proc-card-parent = Запушчаны працэсам
proc-card-user = Карыстальнік
proc-card-state = Стан
proc-card-threads = Патокі
proc-card-memory = Памяць
proc-card-mapped = Адлюстравана
proc-card-bits = Разраднасць
proc-card-started = Запушчаны
proc-card-exe = Выканальны файл
proc-card-cwd = Каталог
proc-card-command = Камандны радок
proc-card-group = Суполка
proc-card-sid = SID
proc-card-priority = Прыярытэт
proc-card-session = Сеанс
proc-card-uptime = Працуе ўжо
proc-card-cpu-user = Час працэсара, уласны
proc-card-cpu-kernel = Час працэсара, у ядры
proc-card-handles = Адкрытыя дэскрыптары
proc-card-faults = Памылкі старонак
proc-card-private = Прыватная памяць
proc-card-swap = Выгружана
proc-card-io-read = Прачытана
proc-card-io-write = Запісана
proc-card-memory-peak = Пік працоўнага набору
proc-card-private-resident = Працоўны набор, прыватны
proc-card-swap-peak = Пік файла падпампоўкі
proc-card-virtual = Віртуальныя байты
proc-card-virtual-peak = Пік віртуальных байтаў
proc-card-paged-pool = Выгружальны пул
proc-card-nonpaged-pool = Нявыгружальны пул
proc-card-io-other = Іншы ўвод-вывад
proc-card-io-read-ops = Аперацыі чытання
proc-card-io-write-ops = Аперацыі запісу
proc-card-io-other-ops = Іншыя аперацыі
proc-kill-title = Завяршэнне працэсу
proc-kill-ask = Завяршыць { $name } [{ $pid }]? Усё, што ён не захаваў, будзе страчана.
proc-killing = Завяршэнне { $name }…
proc-killed = { $name } завершаны
proc-kill-failed = { $name } не ўдалося завяршыць: { $why }
proc-card-env = Асяроддзе
proc-card-modules = Загружаныя модулі
proc-card-version = Версія
proc-card-description = Апісанне
drives-hint = ↑↓ выбар · 0-9 панэлі · Ctrl+1..9 слупкі · F9 усе · Ctrl+R перачытаць
drives-title = Змяніць дыск: што паказваць
drives-col-type = тып
drives-col-device = прыладу
drives-col-label = метку
drives-col-fs = файлавую сістэму
drives-col-size = памеры
drives-col-removable = зменныя дыскі
drives-col-panels = панэлі, якія не з'яўляюцца дыскамі
drives-col-optical = аптычныя дыскі
drives-col-network = сеткавыя рэсурсы
drives-shown = паказваецца: { $what }
drives-hidden = хаваецца: { $what }
drives-reread = дыскі перачытаны
bookmarks-hint = Enter перайсці · Ins захаваць · Del прыбраць · F4 назваць · Esc закрыць

# The search dialog (F7), which asks the whole question in one place.
search-title = Пошук
search-for = Шукаць
search-mode = Як
search-mode-text = тэкст
search-mode-hex = hex
search-mode-regex = рэгулярны выраз
search-case = Улічваць рэгістр
search-words = Толькі поўныя словы
btn-find-next = Знайсці далей
btn-find-prev = Знайсці назад
# The help: the chapters' own names, and the note on a chapter nobody has translated yet
help-chapter-about = Што гэта такое
help-chapter-panels = Панэлі
help-chapter-files = Файлы: капіяванне, перанос, пошук, архівы
help-chapter-viewer = Праграма прагляду
help-chapter-editor = Рэдактар, шаснаццатковая панэль і асэмблер
help-chapter-network = Серверы
help-chapter-config = Канфігурацыя, клавішы і колеры
help-chapter-keys = Клавішы, як яны прызначаны цяпер
help-chapter-commands = Усе каманды і іх клавішы
help-chapter-options = Усе налады і іх значэнні
help-here-panels = Панэлі
help-here-viewer = Праграма прагляду
help-here-editor = Рэдактар
help-here-hex = Шаснаццатковы рэдактар
help-here-dialog = Гэты дыялог
help-in-english = Гэты раздзел яшчэ не перакладзены на вашу мову, таму далей ён па-англійску. Клавішы і налады, якія ён называе, аднолькавыя ва ўсіх мовах.
help-about-byline = FinalCommander { $version } — аўтар: Matteo Baccan
key-ok = OK
key-cancel = Адмена
key-dialog-move = Рухаць
key-tree = Дрэва
key-config = Налады
browse-local-only = файл выбіраецца з гэтага камп'ютара; шлях на серверы ўводзіцца ўручную, і Tab яго дапаўняе
key-browse = Агляд
table-prompt-filter = Радкі, якія змяшчаюць
table-sorted = упарадкавана па { $column }
table-sorted-back = упарадкавана па { $column }, у зваротным парадку
table-unsorted = зноў у парадку, у якім яны ў файле
table-filtered = засталося радкоў: { $rows }
table-unfiltered = зноў усе радкі
table-cut-sorted = з першых { $rows } радкоў — гэта ўсё, што было прачытана
table-too-many = зашмат запісаў, каб парадкаваць ці фільтраваць тут
table-copied =
    { $rows ->
        [one] { $rows } радок у буферы абмену
        [few] { $rows } радкі ў буферы абмену
       *[other] { $rows } радкоў у буферы абмену
    }
table-headed = першы радок — загаловак
table-not-headed = першы радок — даныя: слупкі называюцца па нумарах
table-named-fields = гэты фармат сам называе свае палі: загаловак адгадваць не трэба

# Splitting and combining (SPEC 03 §9)
split-title = Разразанне файла
split-target = Куды
split-size = Кожная частка
split-size-hint = 1440K, 700M, 4G — апошняя частка змяшчае тое, што засталося
split-count = Або на
split-count-hint = частак — лік тут важнейшы за памер
split-bad-count = { $count } не з'яўляецца колькасцю частак: дзве або больш
split-local-file = Разразанне і збіранне працуюць з файлам на гэтым камп'ютары
split-bad-size = «{ $size }» не з'яўляецца памерам: пішыце як 700M або 4G
combine-not-a-piece = Гэта не частка: часткі разрэзанага файла заканчваюцца на .001, .002, … або маюць побач .crc
pieces-working = Разразанне або збіранне: { $done } з { $total }
pieces-cancelled = Спынена, і запісанае прыбрана
split-done = запісана частак: { $parts }, і { $list } побач з імі
combine-done-checked = { $name } сабраны з частак: { $parts }, { $bytes }, і ён адпавядае свайму .crc
combine-done-unchecked = { $name } сабраны з частак: { $parts }, { $bytes } — файла .crc для праверкі не было
pieces-failed = Не выканана: { $error }

# The branch view (SPEC 02 §3d)
branch-reading = Чытанне ўсіх файлаў у { $path }…
branch-title = Галіна: { $path }
branch-done = файлаў: { $count }, з усіх каталогаў ніжэй за гэты · Ctrl+Alt+B вяртае назад
branch-truncated = файлаў: { $count }, і на гэтым абход спыніўся: галіна большая, чым мае сэнс паказваць спісам
branch-not-here = Выгляд галіны датычыцца каталога, а гэтая панэль — спіс

## The release check (SPEC 11 §5).
menu-check-update = Праверыць абнаўленн&і
opt-update-check = Правяраць новыя выпускі раз на дзень
btn-open-page = Адкрыць старонку
update-checking = Запыт да GitHub пра апошні выпуск…
update-title = Абнаўленні
update-available = Выйшаў FinalCommander { $version } (гэта { $current }).
update-available-status = Выйшаў FinalCommander { $version }: Налады → Праверыць абнаўленні
update-current = Гэта апошні выпуск, { $version }.
update-failed = Не ўдалося спытаць пра апошні выпуск: { $why }

## Duplicate files (SPEC 06 §13).
menu-dupes = Пошук файлаў-дублікатаў
dupes-title = Файлы-дублікаты
dupes-recursive = Шукаць у падкаталогах
dupes-min-size = Найменшы файл
dupes-min-size-hint = Памер, як яго пішуць фільтры: 1, 100K, 5M. Пустыя файлы ніколі не параўноўваюцца.
dupes-select = Адзначыць усе копіі, акрамя найстарэйшай
btn-search = Шукаць
dupes-not-here = Пошук дублікатаў пачынаецца з каталога або з адзначанага.
dupes-bad-size = Не памер: { $size }
dupes-looking = Пошук файлаў-дублікатаў…
dupes-progress = Дублікаты: праход { $pass } з 3, { $done }/{ $total }
dupes-none = Сярод файлаў ({ $count }) дублікатаў няма.
dupes-panel = Дублікаты: суполак — { $groups }
dupes-done = суполак дублікатаў: { $groups }, у копіях { $size }.
dupes-truncated = Спынена датэрмінова: суполак пакуль што { $groups }, у копіях { $size }.

## SQLite queries (SPEC 04 §4b).
sql-empty = Напішыце запыт пасля sql:, напрыклад sql: SELECT * FROM t
sql-no-database = sql: патрабуе базу даных SQLite: зайдзіце ў яе або пастаўце на яе курсор.
sql-running = Выкананне запыту…
sql-failed = Запыт не ўдаўся: { $why }

## Asking a language model (SPEC 06 §14).
ai-none = Служба ШІ не наладжана: дадайце блок [[ai]] у fcmd.toml (name, url, model, api, token_env).
ai-title = Спытаць ШІ
ai-no-thread = няма адказу, які можна працягнуць: спачатку спытайце праз `ai:` або пастаўце курсор на адказ у каталогу адказаў
ai-nothing-to-explain = тут няма чаго тлумачыць: адкрыйце файл у праграме прагляду ці рэдактары або спытайце адразу пасля каманды, якая завяршылася памылкай
ai-explain-question = Што гэта такое, з { $name }? Растлумач коратка.
ai-tokens = токенаў: { $in } на ўваходзе, { $out } на выхадзе
ai-none-panel = Службаў ШІ яшчэ няма — F7 дадае
ai-service-removed = { $name } выдалена
ai-probing = Уваход у { $name } на { $host }…
ai-probe-ok = { $host } адказаў HTTP { $status } за { $ms } мс, мадэляў: { $models } — увядзіце пытанне і націсніце Enter
ai-probe-no-model = (мадэлі { $model } сярод іх няма)
ai-row-context = кантэкст
ai-row-conversation = размова
ai-session-title = ШІ: { $name } · { $model } — файлаў у кантэксце: { $files }
ai-context-added = { $count ->
        [one] { $count } файл
        [few] { $count } файлы
       *[other] { $count } файлаў
    } у кантэксце
ai-context-nothing = Няма чаго дадаваць: у кантэкст трапляюць толькі файлы з гэтага дыска
ai-command-ready = каманда стаіць у радку — прачытайце яе, перш чым націскаць Enter
ai-commit-question = Напішы паведамленне коміту для падрыхтаваных змен.
ai-staged-diff = падрыхтаваны diff
ai-send-diff = Падрыхтаваны diff пасылаецца на { $host }
ai-send-files = Файлаў: { $count }, { $size } — будзе паслана на { $host } разам з пытаннем.
ai-asking = Запыт да { $name }…
ai-busy = { $name } думае… { $secs } с · Esc спыняе
ai-conversation-new = Новая размова: яе пачне наступнае пытанне.
ai-live-empty = Увядзіце пытанне ў камандным радку: яно пойдзе да { $name }, а размова будзе з'яўляцца тут.
ai-failed = ШІ не адказаў: { $why }
ai-stopped = спынена: пытанне да { $name } адкінута, і нічога не запісана
ai-context-title = Файлы ў кантэксце
ai-context-all = Усе
ai-context-dropped = { $count ->
        [one] { $count } файл прыбраны з кантэксту
        [few] { $count } файлы прыбраны з кантэксту
       *[other] { $count } файлаў прыбрана з кантэксту
    }
ai-context-hint = Enter або Del: прыбраць з кантэксту — файл застаецца на дыску · F3: што пасылаецца · Esc
ai-threads-title = Размовы
ai-threads-hint = Enter: працягнуць гэтую размову · Esc
ai-thread-resumed = працяг { $name }
ai-next-estimate = далей ≈ { $tokens }
ai-next-over = далей ≈ { $tokens }, больш, чым змяшчае мадэль
ai-send-estimate = Усяго каля { $tokens } токенаў, разам з папярэдняй размовай.
ai-send-estimate-of = Усяго каля { $tokens } токенаў, разам з папярэдняй размовай, з { $max }, якія змяшчае мадэль.
ai-send-over = Усяго каля { $tokens } токенаў: больш за { $max }, якія змяшчае мадэль, таму пачатак можа быць абрэзаны. F8 прыбірае файл.
ai-send-question = Пытанне пасылаецца на { $host }.
btn-send = Паслаць

## Git (SPEC 06 §15).
menu-git = Git
git-title = Git
git-status = Што змянілася
git-log = Журнал
git-blame = Blame { $name }
git-branches = Галіны
git-local-only = git працуе з каталогам на гэтым дыску.
git-running = Запыт да git…
git-failed = git: { $why }
git-clean = Нічога не змянілася.
git-status-title = Зменена: у індэксе { $staged }, па-за індэксам { $unstaged }
git-side-conflict = канфлікт
git-side-staged = у індэксе
git-side-changed = не ў індэксе
git-side-untracked = не адсочваецца
git-kind-modified = зменены
git-kind-added = новы
git-kind-deleted = выдалены
git-kind-renamed = пераназваны
git-kind-copied = скапіяваны
git-kind-type = тып зменены
git-kind-conflict = развязаць
git-as-tree = Паказаць змены дрэвам
git-as-list = Паказаць змены спісам
git-version-head = апошні коміт
git-version-index = у індэксе
git-version-disk = на дыску
git-discard-title = Адкінуць змены
git-discard-body = Адкінуць змены { $count ->
        [one] { $count } файла
       *[other] { $count } файлаў
    }?
git-discard-worktree = назад да індэкса:
git-discard-both = назад да апошняга коміту:
git-discard-added = з індэкса, застаецца на дыску:
git-discard-trash = у сметнік:
git-discard-warning = Тое, што вяртаецца да індэкса ці апошняга коміту, скасаваць нельга.
git-hint = Enter: выбраць · Esc: закрыць

## The editor's character list and word completion.
charmap-title = Сімвалы
charmap-hint = Друкуйце для звужэння (сімвал або яго код, U+00E9) · Enter: уставіць · Esc: назад
editor-no-completion = Ніводнае іншае слова ў файле так не пачынаецца.

## A stored session's address, copied (NetBox's Generate URL).
net-url-copied = Скапіявана { $url }

## The system log (SPEC 06 §16).
menu-syslog = Сістэмны журнал
syslog-reading = Чытанне сістэмнага журнала…

## Deleting lines by pattern (FAR #10).
editor-prompt-delete-matching = Прыбраць радкі, якія адпавядаюць (! напачатку: пакінуць толькі іх)
editor-lines-deleted = Прыбрана радкоў: { $count }

## The copy dialog's size policy (FAR #493).
copy-conflict-size = перазапісаць, калі памер адрозніваецца

# Copy dialog: what an error does (FAR's issue #597)
copy-on-error = Калі файл не ўдаецца скапіяваць
copy-error-ask = Спытаць
copy-error-skip = Прапусціць і працягнуць
copy-error-stop = Спыніць капіяванне

# Attributes dialog: the times copied from one file to another (FAR's issue #241)
attr-times-copied = Час скапіяваны: Ctrl+Shift+V устаўляе яго ў дыялог іншага файла
attr-times-none = Яшчэ нічога не скапіявана: спачатку Ctrl+Shift+C у дыялогу файла
attr-times-pasted = Час устаўлены: OK яго ўсталюе

# Editor: replacements kept by name and recent ones (FAR's issue #282)
replace-tpl-title = Замены
replace-tpl-hint = Enter выконвае · Ins захоўвае нядаўнюю пад імем · Del забывае захаваную · Esc
replace-tpl-name = Захаваць гэтую замену як
replace-tpl-kept = Захавана як {$name}
replace-tpl-empty = Замен яшчэ няма: спачатку Ctrl+F7

# Panel settings: a view and a sort per folder (FAR's issue #389)
opt-remember-view = Кожны каталог мае свой выгляд і парадкаванне

# Session editor: a cap on a copy's speed to or from the site
site-max-kbps = Абмежаванне хуткасці, КБ/с (пуста: без абмежавання)

# Session editor: IMAP, read only
site-protocol-imap = IMAP (пошта, толькі чытанне)
site-protocol-imaps = IMAPS (пошта праз TLS, толькі чытанне)

# The AI services as a list, like the sessions
ai-services-title = Службы ШІ
ai-services-hint = Enter: да яе звяртаецца ai: · Ins новая · F4 змяніць · Del выдаліць · Esc
ai-service-title = Служба ШІ
ai-field-name = Імя
ai-field-api = Пратакол
ai-field-url = Адрас
ai-field-model = Мадэль
ai-field-token-env = Ключ у зменнай
ai-field-token-note = Пуста: ключ запытваецца раз за сеанс і ніколі не запісваецца.
ai-field-system = Што ёй кажуць спачатку
ai-service-incomplete = Службе патрэбны імя і адрас
ai-service-default = ai: цяпер звяртаецца да {$name}
ai-key-prompt = Ключ для {$name} (захоўваецца да выхаду)
ai-api-openai = OpenAI і сумяшчальныя
ai-api-anthropic = Anthropic
ai-api-ollama = Ollama (на гэтым камп'ютары)
menu-ai = Службы ШІ

# The secrets file
vault-password = Пароль файла сакрэтаў fcmd
vault-new = Захаваць гэты ключ у зашыфраваным файле сакрэтаў fcmd: выберыце яго пароль (пуста: толькі на гэты сеанс)
vault-refused = Файл сакрэтаў не адкрыўся: {$why}

# The programs from elsewhere
drive-tools = Вонкавыя праграмы
drive-ai = ШІ
drive-git = Git
tools-found = знойдзена
tools-missing = не ўсталявана
tools-summary = знойдзена вонкавых праграм: { $found } з { $total }; F3 кажа, для чаго кожная, F4 усталёўвае адсутную
tools-card = Вонкавая праграма
tools-card-program = Праграма
tools-card-where = Дзе
tools-card-version = Версія
tools-card-for = Для чаго
tools-card-install = Як усталяваць
tools-card-why = Чаму fcmd запускае яе, а не робіць гэта сам: F1, «Староннія праграмы».
tools-install-any = усталюйце пакет, які дае { $name }, і пераканайцеся, што ён ёсць у PATH
tools-already = { $name } ужо тут ёсць
tools-for-archives = архівы: { $what }
tools-for-decompile = Shift+F6 у праграме прагляду для { $what }
tools-for-git = панэль Git (Ctrl+Alt+G) і галіна ў запрашэнні
tools-for-video = кадры відэа ў праграме прагляду і хуткім праглядзе (F3, Ctrl+Q)
tools-for-open = адкрыццё спасылкі ў браўзеры
tools-for-syslog = Каманды → Сістэмны журнал
tools-for-services = панэль службаў
tools-card-site = Вэб-сайт
tools-no-package = Пра ніводзін менеджар пакетаў на гэтым камп'ютары не вядома, што ён мае { $name }. Адкрыць старонку спампоўкі?
tools-install-ask = Усталяваць { $name }, выканаўшы ў тэрмінале:
tools-install-how = Менавіта ў тэрмінале sudo пытаецца ваш пароль. Ctrl+Alt+E вяртае да панэляў, а Ctrl+R у гэтай панэлі правярае зноў.
tools-install-else = Калі не ўдасца: { $url }
tools-install-failed = fcmd: { $name } не ўсталявана. Спампуйце з { $url }
tools-installing = усталяванне { $name } у тэрмінале: Ctrl+Alt+E вяртае да панэляў, Ctrl+R там правярае зноў
btn-install = Усталяваць

# Plugins configuration
plugins-config-title = Дадаткі
plugins-config-hint = Enter: уключыць або выключыць · F3 што гэта і што ён дае · Esc
plugins-broken = не загрузіўся — F3 кажа чаму
plugins-cannot-toggle = дадатак, які не загрузіўся, нельга ўключыць: F3 кажа, што з ім не так
plugins-on = { $id } уключаны: яго каманды цяпер у Ctrl+Shift+F11
plugins-on-restart = { $id } уключаны: яго каманды цяпер у Ctrl+Shift+F11, астатняе з'явіцца з наступнага запуску
plugins-off = { $id } выключаны: яго каманды зніклі з Ctrl+Shift+F11
plugins-off-restart = { $id } выключаны: яго каманды зніклі з Ctrl+Shift+F11, астатняе знікне з наступнага запуску
plugins-state-on = укл
plugins-state-off = выкл
plugins-card-id = Ідэнтыфікатар
plugins-card-name = Імя
plugins-card-version = Версія
plugins-card-author = Аўтар
plugins-card-about = Апісанне
plugins-card-dir = Каталог
plugins-card-state = Стан
plugins-card-brings = Дае
plugins-card-commands = Каманды (Ctrl+Shift+F11):
plugins-card-notes = Сказана падчас загрузкі:
plugins-none = дадаткі не знойдзены · яны жывуць у { $dir }, кожны ў сваім каталогу з plugin.toml

# The git panel
git-file-log = Журнал { $name }
git-commit = Закаміціць падрыхтаванае…
git-pull = Pull
git-push = Push
git-pushing = Выконваецца push…
git-pulling = Выконваецца pull…
git-commit-prompt = Паведамленне коміту
git-commit-empty = Коміту патрэбна паведамленне.
git-already-on = { $branch } — гэта галіна, на якой вы цяпер.
git-switch-ask = Перайсці на { $branch }? Файлы, у якіх галіны разыходзяцца, зменяцца на дыску.
git-done = git: { $said }
git-branches-title = Галіны: { $count }
git-log-title = Журнал: { $count }
git-log-of-title = Журнал { $of }: { $count }
btn-switch = Перайсці
key-git = Git
key-git-diff = Diff
key-git-stage = Індэкс
key-git-unstage = ЗІндэк
key-git-discard = Адкін
key-git-commit = Коміт
key-git-show = Паказ
key-git-log = Журнал
key-git-changes = Змены
key-git-pull = Pull
key-git-push = Push
key-git-branches = Галіны

undo-title = Скасаванне
btn-undo = Скасаваць
menu-undo = Скасаваць апошнюю аперацыю
menu-strip-meta = Прыбраць метаданыя выяў…
undo-nothing = няма чаго скасоўваць: ад запуску праграмы не было капіявання, пераносу, выдалення ў сметнік ці новага каталога
undo-copy =
    { $count ->
        [1] Паслаць у сметнік файл, створаны капіяваннем:
        [one] Паслаць у сметнік { $count } файл, створаны капіяваннем:
        [few] Паслаць у сметнік { $count } файлы, створаныя капіяваннем:
       *[other] Паслаць у сметнік { $count } файлаў, створаных капіяваннем:
    }
undo-move =
    { $count ->
        [1] Перанесці файл назад, дзе ён быў:
        [one] Перанесці { $count } файл назад, дзе ён быў:
        [few] Перанесці { $count } файлы назад, дзе яны былі:
       *[other] Перанесці { $count } файлаў назад, дзе яны былі:
    }
undo-trash =
    { $count ->
        [1] Вярнуць файл са сметніка:
        [one] Вярнуць { $count } файл са сметніка:
        [few] Вярнуць { $count } файлы са сметніка:
       *[other] Вярнуць { $count } файлаў са сметніка:
    }
undo-mkdir = Выдаліць новы каталог, калі ён дагэтуль пусты:
strip-title = { $count ->
        [one] Метаданыя { $count } выявы
       *[other] Метаданыя { $count } выяў
    }
strip-all = Усё — тэкст, EXIF, XMP, IPTC, каментарыі, C2PA
strip-ai = Толькі тое, што запісала мадэль выяў — промпт, workflow, налады
strip-camera = Усё, акрамя EXIF камеры і аўтарскіх правоў — без GPS
strip-hint = Enter: прыбраць; выява не перакадоўваецца, Ctrl+Alt+Z вяртае назад · Esc
strip-nothing = ніводзін PNG, WebP ці JPEG на гэтым дыску не адзначаны і не стаіць пад курсорам
strip-done = { $count ->
        [one] { $count } выява
        [few] { $count } выявы
       *[other] { $count } выяў
    } без метаданых (прыбрана блокаў: { $blocks }), не мелі чаго прыбіраць: { $clean } · Ctrl+Alt+Z вяртае іх
undo-restore = { $count ->
        [1] Вярнуць выяву з яе метаданымі:
        [one] Вярнуць { $count } выяву з метаданымі:
        [few] Вярнуць { $count } выявы з метаданымі:
       *[other] Вярнуць { $count } выяў з метаданымі:
    }
undo-done =
    { $count ->
        [one] { $count } імя зноў там, дзе было
        [few] { $count } імені зноў там, дзе былі
       *[other] { $count } імёнаў зноў там, дзе былі
    }
slow-drives = спіс дыскаў заняў { $secs } с
slow-drives-late = спіс дыскаў чакаў { $secs } с: { $drives } не адказалі своечасова і паказаны без падрабязнасцей — сеткавы дыск, сервера якога няма?
slow-listing = чытанне { $what } заняло { $secs } с
# editor search and undo (0.41)
btn-find-all = Усе супадзенні
btn-replace-all = Замяніць усё
btn-replace-one = Замяніць наступнае
editor-replace-title = Замена
editor-replace-style = Захоўваць стыль (foo → bar, Foo → Bar, FOO → BAR)
editor-replace-groups = У рэгулярным выразе $1 і { "${name}" } — тое, што злавіла група, $$ — долар
editor-matches-title = Радкі з `{ $query }`: { $count }
editor-matches-hint = Друкуйце для звужэння · Enter: перайсці да радка · Esc: назад


# sorting and extended attributes (0.41)
opt-sort-case = Вялікія і малыя літары ў парадкаванні
opt-sort-case-auto = як у гэтай сістэме
opt-sort-case-yes = адрозніваць
opt-sort-case-no = адзін алфавіт
copy-keep-xattrs = Захоўваць пашыраныя атрыбуты

# editor conversions, disk diff and guides (0.41)
editor-convert-title = Запісаць файл як
editor-convert-hint = Друкуйце для звужэння · Enter: пераўтварыць (F2 запісвае, Ctrl+Z вяртае) · Esc: назад
editor-convert-eol = Канец радка: { $eol }
editor-convert-bom-on = Метка парадку байтаў (BOM): запісваецца
editor-convert-bom-off = Метка парадку байтаў (BOM): не запісваецца
editor-convert-page = Кадыроўка: { $page }
editor-converted-eol = { $name }: пры наступным захаванні радкі будуць заканчвацца на { $what }
editor-converted-bom-on = { $name }: наступнае захаванне запіша метку парадку байтаў
editor-converted-bom-off = { $name }: наступнае захаванне не запіша меткі парадку байтаў
editor-converted-page = { $name }: наступнае захаванне запіша { $what }
editor-convert-same = Файл ужо запісаны менавіта так
editor-convert-no-bom = { $page } не мае меткі парадку байтаў: яна ёсць толькі ў UTF-8 і UTF-16
editor-convert-lossy = { $page } не змяшчае ўсіх сімвалаў { $name }: нічога не зменена
btn-reload = Перачытаць
btn-what-changed = Што змянілася
editor-diff-buffer = у рэдактары
editor-diff-hint = Гэтае параўнанне толькі паказвае, што змянілася — Esc вяртае да пытання


# job journal and saved syncs (0.41)
journal-left = Капіяванняў, не завершаных мінулым сеансам: { $count } · Alt+Shift+F6 паказвае спіс
journal-title = Незавершаныя заданні
journal-hint = Enter працягнуць · Del адкінуць · Esc закрыць
journal-none = Незавершаных заданняў не засталося
journal-row = { $kind }: { $what } → { $target } · файлаў гатова: { $done }
journal-row-unreadable = не чытаецца: { $file } ({ $error })
journal-unreadable = { $file }: гэта не журнал заданняў, які ўмее чытаць гэтая версія ({ $error }). Alt+Shift+F6 паказвае яго, а Del там выдаляе
journal-resumed = Працяг { $kind }: файлаў ужо было гатова: { $done }
journal-discarded = Незавершанае заданне адкінута разам з тым, што яно запісала напалову
journal-cannot-resume = Гэты журнал не чытаецца; Del яго выдаляе
menu-jobs-left = Незавершаныя заданні
sync-report = Запісаць справаздачу пра выкананне ў (файл)
sync-name = Захаваць гэтую пару як (імя)
sync-saved-button = Захаваныя…
sync-saved-title = Захаваныя сінхранізацыі
sync-saved-hint = Enter выканаць · F4 змяніць · Del забыць · Esc закрыць
sync-saved-none = Ніводная сінхранізацыя яшчэ не захавана: дайце ёй імя ў дыялогу (Ctrl+Shift+Y)
sync-saved-removed = { $name } больш не захоўваецца
sync-report-local = Справаздача павінна быць файлам на гэтым камп'ютары
sync-report-written = Справаздача пра сінхранізацыю — у { $path }
sync-report-not-written = Справаздачу не ўдалося запісаць у { $path }: { $error }
sync-report-title = Справаздача пра сінхранізацыю
sync-report-name = Імя: { $name }
sync-report-left = Левая: { $path }
sync-report-right = Правая: { $path }
sync-report-to-right = Скапіявана злева → направа: { $count }
sync-report-to-left = Скапіявана справа → налева: { $count }
sync-report-cut-right = Прыбрана справа: { $count }
sync-report-cut-left = Прыбрана злева: { $count }
sync-report-skipped = Пакінута як ёсць, хоць адрозніваюцца: { $count }
sync-report-failed = Не ўдалося: { $count }
sync-report-stopped = Спынена на паўдарозе, таму маглі быць выкананы або не: { $count }
sync-report-not-done = Не выканана, бо папярэдні крок не ўдаўся або быў спынены: { $count }
menu-sync-saved = Захаваныя сінхранізацыі

# hex editor search
hex-search-wrapped = `{ $query }`: знойдзена, пачаўшы зноў з іншага канца файла
hex-replace-title = Замена
hex-replace-with = Замяніць на
btn-replace = Замяніць
hex-replace-ask = Замяніць { $was } у пазіцыі { $at } на { $with }?
hex-replace-length = замена мае даўжыню { $with } байт, а супадзенне — { $was }: гэта зрушыла б усё пасля яго — спачатку націсніце Ins для рэжыму ўстаўкі

# pixels and video (0.41)
opt-viewer-pixels = Выявы
opt-viewer-pixels-auto = сапраўдныя пікселі там, дзе тэрмінал іх малюе
opt-viewer-pixels-kitty = графічны пратакол kitty
opt-viewer-pixels-iterm = убудаваныя выявы iTerm2
opt-viewer-pixels-sixel = sixel
opt-viewer-pixels-off = заўсёды паўблокі
video-needs-program = гэта відэа ({ $what }): яго кадры малююцца праз `{ $program }`, якая на гэтым камп'ютары не ўсталявана — Shift+F11 усё роўна чытае кантэйнер
video-no-frame = з гэтага відэа няма кадра: { $why }
video-playing = прайграванне з хуткасцю { $fps } кадраў за секунду, без гуку — Space ставіць на паўзу
video-paused = паўза на { $at } — Space працягвае, PgUp і PgDn крочаць на дзясятую частку
video-ended = відэа скончылася — Space прайграе яго зноў


# finding things
menu-saved-searches = Захаваныя пошукі
drive-searches = Захаваныя пошукі
find-keys = F2 захоўвае гэты пошук пад імем · F9 захаваныя
find-save-name = Захаваць гэты пошук як
find-saved = пошук захаваны як { $name }
find-saved-none = ніводзін пошук яшчэ не захаваны: F2 у дыялогу пошуку захоўвае
find-saved-title = Захаваныя пошукі — Enter адкрывае, F4 змяняе, Del забывае
find-saved-running = пошук { $name }…
find-saved-gone = пошук { $name } забыты
find-loaded = пошук { $name } загружаны
find-fuzzy = Прыблізнае імя: літары вышэй, па парадку, найлепшае супадзенне першым
find-meta = Метаданыя (artist=bowie; camera=)
find-index = Спытаць сістэмны індэкс замест абходу (можа прапусціць новыя файлы)
tools-for-index = індэкс, які Alt+F7 можа спытаць замест абходу
menu-frecent = Часта наведваныя каталогі
frecent-none = ніводзін каталог яшчэ не наведаны
frecent-gone = { $path } прыбраны з часта наведваных каталогаў
frecent-hint = друкуйце літары шляху · Enter перайсці туды · Del забыць · Esc закрыць

# ai third round
ai-redact-off = Маскаванне выключана: усё пасылаецца як ёсць
ai-redact-none = Сакрэтаў для маскавання не знойдзена
ai-redact-count = Сакрэтаў замаскавана перад пасылкай: { $count } ({ $kinds })
ai-redacted = сакрэтаў замаскавана: { $count }
ai-preview-title = Што пасылаецца да { $name } ({ $host })
ai-preview-instruction = Інструкцыя
ai-preview-earlier = Папярэдняя размова
ai-preview-question = Пытанне і яго файлы
ai-preview-key = F3 паказвае, што пасылаецца
ai-preview-reading = Чытанне таго, што было б паслана…
key-ai-redact = Маск.
ai-redact-now-on = Маскаванне ўключана: ключы, токены, прыватныя ключы і паролі маскуюцца, перш чым нешта пасылаецца
ai-redact-now-off = Маскаванне выключана: тое, што пасылаецца, ідзе як ёсць
ai-panel-redact-on = сакрэты замаскаваны
ai-panel-redact-off = сакрэты не замаскаваны
key-ai-local = Лакал.
ai-row-refused = не гэты камп'ютар: адхілена
ai-panel-local-only = толькі лакальна
ai-local-now-on = Рэжым «толькі лакальна» ўключаны: служба, якая не на гэтым камп'ютары, адхіляецца, і ёй нічога не пасылаецца
ai-local-now-off = Рэжым «толькі лакальна» выключаны: можна звяртацца да любой службы са спіса
ai-local-refused = Рэжым «толькі лакальна» ўключаны: { $name } адказвае на { $host }, а гэта не гэты камп'ютар — нічога не паслана (F6 у панэлі ШІ выключае яго)
ai-rename-prompt = Якімі павінны стаць імёны ({ $count })? (словамі)
ai-send-names = Інструкцыя і { $count } з { $total } імёнаў — толькі імёны, нічога з файлаў — пасылаюцца на { $host }.
ai-rename-ready = шаблон мадэлі стаіць у дыялогу: прачытайце папярэдні прагляд, выпраўце памылковае, і толькі OK пераназывае
ai-rename-waiting = шаблон пераназывання ад мадэлі гатовы: Ctrl+Shift+F6 адкрывае дыялог з ім
ai-rename-unreadable = мадэль не адказала шаблонам пераназывання: яе адказ у каталогу адказаў (ai:), і нічога не пераназвана
ai-rewrite-prompt = Што зрабіць з вылучаным блокам? (словамі)
ai-rewrite-no-block = спачатку вылучыце блок (Shift+стрэлкі): перапісванне замяняе менавіта вылучанае, а блок-слупок не мае формы, у якую можна вярнуцца
ai-rewrite-read-only = гэты файл тут толькі для чытання: перапісанаму не было б куды трапіць
ai-rewrite-before = блок
ai-rewrite-after = як яго перапісала мадэль
ai-rewrite-hint = Перапісанае мадэллю супраць блока — Enter ставіць яго на месца блока, Esc пакідае блок як ёсць
diff-hint-rewrite = Enter замяніць блок · Esc пакінуць
ai-rewrite-applied = блок заменены перапісаным мадэллю — адзін Ctrl+Z вяртае яго
ai-rewrite-left = блок пакінуты як ёсць
ai-rewrite-stale = блок ужо не той, якім быў, калі запытвалі перапісванне: нічога не заменена — версія мадэлі ў каталогу адказаў (ai:)
ai-rewrite-same = мадэль вярнула блок без змен: замяняць няма чаго
ai-answer-kept = адказ — { $name } у каталогу адказаў (ai:) — рэдактар пакінуты наперадзе
ai-failure-question = Гэтая каманда завяршылася з кодам { $code }: `{ $command }` — чаму яна не ўдалася і што гэта выправіць? Коратка.
ai-send-failure = Каманда, якая не ўдалася, і апошнія радкі яе вываду ({ $lines }) пасылаюцца на { $host }.

# archives and jobs
arc-test-whole = { $name }: праверана запісаў: { $files }, { $bytes } — усе цэлыя
arc-test-bad-title = { $name }: { $bad } з { $files } запісаў аказаліся пашкоджанымі

# extending it
plugin-column-failed = { $column }: { $error } — яго слупок застаецца пустым для файлаў, за якія ён не адказаў
plugins-card-columns = Слупкі (як іх называе [[column_set]]):
usermenu-far-row = Што прачытана інакш, чым у FarMenu.ini ({ $count })
usermenu-far-notes = меню паходзіць з FarMenu.ini ад FAR: прачытана інакш рэчаў: { $count } — апошні радок іх пералічвае
usermenu-far-not-run = Нічога не запушчана.
find-age = Зменена, дзён таму (7, >=30, 7-30)
find-bad-age = `{ $text }` не з'яўляецца ўзростам у днях — { $error }
find-label-age = { $days } дз
find-keys-ai = Ctrl+Alt+Q апісвае пошук словамі: моўная мадэль запаўняе гэтыя радкі
ai-find-prompt = Што вы шукаеце? (словамі)
ai-send-words = Гэтыя словы — і нічога з панэляў: ні імя, ні шляху, ні файла — пасылаюцца на { $host }.
ai-find-ready = пошук ад мадэлі стаіць у дыялогу: прачытайце радкі, выпраўце памылковае, і толькі OK шукае
ai-find-waiting = пошук ад мадэлі гатовы: Alt+F7 адкрывае дыялог з ім
ai-find-unreadable = мадэль не адказала пошукам: яе адказ у каталогу адказаў (ai:), і нічога не шукалася
ai-find-left-out = { $said } — НЕ ў радках, бо яны не могуць гэтага выказаць: { $what }
# treemap and compare
usage-map-title = Выкарыстанне дыска, усё дрэва: { $path }
usage-map-walking = Вымярэнне… файлаў: { $files }, пакуль што { $size }
usage-map-where = { $path } · { $size } · файлаў: { $files }
usage-map-unreadable = непрачытаных каталогаў: { $count }
usage-map-rest = меншых файлаў: { $count }
usage-map-others = іншых: { $count }
usage-map-empty = У гэтым каталогу нішто не займае месца
usage-map-chosen = { $name } · { $size } · { $share }%
usage-map-hint = ←↑↓→ · Enter унутр · BkSp уверх · Tab спіс · G панэль туды · Esc
usage-map-hint-list = ↑↓ · Enter унутр · BkSp уверх · Tab карта · G панэль туды · Esc
usage-map-hint-walking = Esc спыняе
usage-map-top = Гэта каталог, які вымяралі: вышэй за яго тут нічога няма
usage-map-no-file = Гэты радок — шмат малых файлаў, а не адзін, да якога можа перайсці панэль
bin-comparing = Параўнанне двух файлаў байт за байтам… Esc спыняе
bin-bytes = байтаў: { $count }
bin-same = Абодва файлы аднолькавыя, байт у байт: па { $bytes } байт
bin-where = адрозненне { $at } з { $count } · першае на зруху { $offset } · байтаў, якія адрозніваюцца: { $differing }
bin-hint = Alt+↑↓ адрозненні · ↑↓ PgUp PgDn Home End радкі · Esc закрыць
diff-unified-on = Адзін слупок: што пайшло, потым што прыйшло
diff-unified-off = Два слупкі, побач
diff-patch-title = Захаваць адрозненні як патч (.diff)
diff-patch-saved = { $name } запісаны: адзіны diff, радкоў: { $lines }
diff-patch-nothing = Абодва файлы аднолькавыя: патча для захавання няма
# wastebasket and external editor
drive-trash = Сметнік
drive-trash-closed = Сметнік (гэтая сістэма не дае праграме яго прагледзець)
menu-trash = Сметнік
menu-trash-empty = Ачысціць сметнік
key-trash-restore = Вярнуць
key-trash-empty = Ачысц
btn-empty = Ачысціць
btn-rename = Пераназваць
trash-closed = сметнік гэтай сістэмы можна адкрыць толькі з яе ўласнага файлавага менеджара
trash-empty = сметнік пусты
trash-failed = сметнік: { $why }
trash-restored =
    { $count ->
        [one] { $count } элемент зноў там, дзе быў
        [few] { $count } элементы зноў там, дзе былі
       *[other] { $count } элементаў зноў там, дзе былі
    }
trash-purged =
    { $count ->
        [one] { $count } элемент выдалены назаўсёды
        [few] { $count } элементы выдалены назаўсёды
       *[other] { $count } элементаў выдалена назаўсёды
    }
trash-card-name = Імя
trash-card-from = Быў у
trash-card-deleted = Выдалены
trash-card-size = Памер
trash-clash-title = Аднаўленне
trash-clash-body = { $path } ужо існуе.
trash-clash-note = «Замяніць» пасылае наяўны ў сметнік; «Пераназваць» вяртае гэты побач з ім.
trash-clash-more =
    { $count ->
        [one] Яшчэ { $count } элемент у такім самым становішчы.
        [few] Яшчэ { $count } элементы ў такім самым становішчы.
       *[other] Яшчэ { $count } элементаў у такім самым становішчы.
    }
trash-purge-title = Выдаленне назаўсёды
trash-purge-body =
    { $count ->
        [1] Выдаліць { $name } са сметніка?
        [one] Выдаліць { $count } элемент са сметніка?
        [few] Выдаліць { $count } элементы са сметніка?
       *[other] Выдаліць { $count } элементаў са сметніка?
    }
trash-purge-note = Гэта выдаляе канчаткова: вярнуць іх няма чым.
trash-empty-title = Ачыстка сметніка
trash-empty-body = Выдаліць усё, што ёсць у сметніку?
opt-editor-external = Вонкавы рэдактар
opt-editor-use-external = Ужываць вонкавы рэдактар для F4 (тады Alt+F4 — убудаваны)
opt-viewer-external = Вонкавая праграма прагляду
opt-viewer-use-external = Ужываць вонкавую праграму прагляду для F3 (тады Alt+F3 — убудаваная)


# editor features
opt-editor-line-numbers = Нумараваць радкі
editor-line-numbers-on = Нумары радкоў уключаны
editor-line-numbers-off = Нумары радкоў выключаны
editor-prompt-pipe-block = Прапусціць блок праз каманду:
editor-prompt-pipe-file = Прапусціць увесь файл праз каманду:
editor-prompt-insert-output = Уставіць тое, што выводзіць гэтая каманда:
editor-prompt-insert-file = Уставіць файл:
editor-prompt-write-block = Запісаць блок у файл:
editor-pipe-running = Выконваецца { $command } …
editor-pipe-done = { $command }: заменена яе вывадам, радкоў: { $lines } (Ctrl+Z вяртае)
editor-pipe-inserted = { $command }: устаўлена радкоў: { $lines }
editor-pipe-empty = { $command } нічога не вывела: нічога не ўстаўлена
editor-pipe-failed = { $command }: { $error } — нічога не зменена
editor-pipe-changed = { $command }: тэкст змяніўся, пакуль каманда працавала — нічога не заменена
editor-pipe-gone = { $command }: файл, для якога яе запускалі, ужо не адкрыты — нічога не зменена
editor-pipe-column = Блок-слупок нельга прапусціць праз каманду: вылучыце цэлыя радкі або нічога — для ўсяго файла
editor-file-inserted = { $name } устаўлены, радкоў: { $lines }
editor-insert-not-local = З рэдактара можна прачытаць або запісаць толькі файл на гэтым камп'ютары
editor-insert-too-big = завялікі, каб прачытаць у рэдактар
editor-block-written = Блок запісаны ў { $name }: { $size }
editor-block-write-self = { $name } — гэта файл, які рэдагуецца: блок не запісаны паверх яго
editor-choice-hint = Друкуйце для звужэння · Enter: выбраць · Esc: назад
editor-syntax-title = Расфарбаваць файл як
editor-syntax-auto = Як кажа яго імя: { $language }
editor-syntax-auto-none = Як кажа яго імя: нічога вядомага
editor-syntax-none = Звычайны тэкст, без колераў
editor-syntax-set = { $name } цяпер чытаецца як { $language }
editor-syntax-off = { $name } цяпер чытаецца як звычайны тэкст
editor-case-heading = Змяніць рэгістр
editor-case-upper = ВЯЛІКІЯ ЛІТАРЫ
editor-case-lower = малыя літары
editor-case-title = Як У Загалоўку: вялікая літара напачатку кожнага слова
editor-case-toggle = іНВЕРСІЯ: вялікія становяцца малымі, а малыя — вялікімі
editor-no-word = Нічога не вылучана, і курсор не стаіць на слове
editor-prompt-isearch = Пошук падчас уводу (↓ ↑ наступны і папярэдні, Esc вяртае)
editor-word-only = Слова пад курсорам больш нідзе не сустракаецца
editor-not-on-word = Курсор не стаіць на слове
editor-prompt-reflow = На колькі слупкоў звярстаць абзац? (толькі Enter: { $width })
editor-reflowed = Абзац звярстаны на { $width } слупкоў (Ctrl+Z вяртае)
editor-reflow-bad-width = { $width } не з'яўляецца шырынёй: колькасць слупкоў ад 8 да 1000
editor-no-paragraph = Курсор стаіць на пустым радку: абзаца тут няма
editor-no-name-yet = Гэты файл яшчэ не мае імя: спачатку захавайце яго
editor-tools-title = Каманды рэдактара
editor-tool-isearch = Пошук падчас уводу
editor-tool-word-next = Наступнае месца, дзе сустракаецца слова пад курсорам
editor-tool-word-prev = Папярэдняе месца, дзе сустракаецца слова пад курсорам
editor-tool-pipe = Прапусціць блок або ўвесь файл праз каманду
editor-tool-write-block = Запісаць блок у файл
editor-tool-case = Змяніць рэгістр блока
editor-tool-reflow = Пераверстаць абзац на зададзеную шырыню
editor-tool-insert-file = Уставіць файл у пазіцыю курсора
editor-tool-insert-output = Уставіць тое, што выводзіць каманда
editor-tool-insert-date = Уставіць дату і час
editor-tool-insert-name = Уставіць імя гэтага файла
editor-tool-insert-path = Уставіць поўны шлях гэтага файла
editor-tool-line-numbers = Нумары радкоў уключыць або выключыць
editor-tool-syntax = Выбраць мову, якой расфарбоўваецца файл
