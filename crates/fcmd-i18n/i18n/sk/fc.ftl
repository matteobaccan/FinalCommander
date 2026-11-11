# Key bar labels: short, they must fit in a few columns.
key-help = Pomoc
key-menu = UžMenu
key-view = Zobraz
key-edit = Uprav
key-copy = Kopír.
key-print = Tlač
key-move = Premen
key-mkdir = VytvPr
key-delete = Zmazať
key-pulldown = KonfMn
key-quit = Koniec
# The key bar while a modifier is held: FAR's own words, six columns wide.
key-add = Spakov
key-extract = Rozpak
key-arccmd = PríkPk
key-edit-new = Uprav…
key-rename = Premen
key-group = Skupin
key-selup = VybPrv
key-left = Ľavý
key-right = Pravý
key-sort-name = Názov
key-sort-ext = Prípon
key-sort-write = Zmena
key-sort-size = Veľkos
key-sort-unsort = Netrie
key-sort-create = Vytvor
key-sort-access = Prístu
key-sort = Tried
key-mklink = Prepoj
key-find = Hľadať
key-history = Histor
key-folders = HsPrie
key-plugins = Moduly

# Panel chrome
list-up = Nahor
list-folder = Priečinok
col-name = Názov
col-ext = Príp
col-size = Veľkosť
col-allocated = Alokované
col-modified = Zmenené
col-created = Vytvorené
col-accessed = Prístup
col-attr = Atrib
col-owner = Vlastník
col-group = Skupina
col-descr = Popis
col-links = Prp
col-git = Git
descr-title = Popis súboru { $name }
descr-set = popis pre { $name } uložený
descr-cleared = popis pre { $name } odstránený
descr-not-here = popisy sa ukladajú vedľa priečinka na tomto počítači
col-date-time = Dátum      Čas
panel-list-count = položiek v zozname: { $count }
panel-empty = (prázdne)
panel-loading = čítam…

# Prompts
prompt-select-mask = Označiť súbory podľa masky:
prompt-deselect-mask = Odznačiť súbory podľa masky:
prompt-goto = Prejsť do priečinka:
prompt-filter = Maska súborov alebo časť názvu:
prompt-search = Rýchle hľadanie:

# Dialogs
dialog-error = Chyba
dialog-help-hint = Esc späť na obsah · F7 hľadá vo všetkých kapitolách · F1 zatvorí · ↑↓ PgUp PgDn posun
dialog-help-index = FinalCommander { $version } — Pomocník
dialog-help-hint-index = Enter otvorí kapitolu · F7 hľadá vo všetkých kapitolách · Esc zatvorí
help-search-prompt = Hľadať v pomocníkovi
help-search-none = Žiadny riadok pomocníka neobsahuje „{ $query }“
help-search-title = „{ $query }“ v pomocníkovi: riadkov { $count }
output-search-prompt = Hľadať vo výstupe
output-search-empty = Zatiaľ sa nič nevypísalo
output-search-found = Riadok { $line } z { $lines }
output-search-wrapped = Riadok { $line } z { $lines }, znova od najnovšieho
output-search-none = Žiadny riadok výstupu neobsahuje „{ $query }“

# Status messages
msg-not-implemented = { $command } v tomto zostavení ešte nie je implementovaný
msg-selected = označených položiek: { $count }
msg-deselected = odznačených položiek: { $count }
msg-mask-invalid = neplatná maska: { $error }
msg-theme = téma: { $name }
msg-font-bigger = písmo: terminál bol požiadaný o väčšie
msg-font-smaller = písmo: terminál bol požiadaný o menšie
msg-font-reset = písmo: terminál bol požiadaný o pôvodné
msg-sort = triedenie: { $key }{ $reverse }
msg-sort-groups-on = skupiny triedenia zapnuté ({ $count })
msg-sort-groups-off = skupiny triedenia vypnuté ({ $count })
msg-sort-groups-none = nie sú určené žiadne skupiny triedenia: pozri [[sort_group]] v konfigurácii
msg-view = zobrazenie: { $mode }
msg-hidden-on = skryté súbory sa zobrazujú
msg-hidden-off = skryté súbory sa nezobrazujú
msg-no-history = história je na konci
msg-tab-last = panel má vždy aspoň jednu kartu
msg-tab-end = karta je už na tomto konci radu
tabs-title = Karty
msg-tab-moved = karta sa presunula do druhého panela
msg-tab-locked = táto karta je uzamknutá na svoj priečinok
msg-tab-locked-on = karta uzamknutá na svoj priečinok
msg-tab-locked-off = karta odomknutá
msg-bookmark-set = skratka { $digit }: { $path }
msg-bookmark-unset = skratka { $digit } nie je nastavená
msg-bookmark-added = { $path } uložený
msg-bookmark-removed = { $path } odstránený
msg-bookmark-known = tento priečinok už v zozname je
msg-dirsize = { $path }: { $bytes }, súborov: { $files }
msg-dirsize-many = meriam priečinky ({ $count })…
usage-title = Využitie disku: { $path } · { $size } · Tab: celý strom ako mapa
usage-not-here = využitie priečinka sa meria na disku, nie v archíve ani v zozname nájdených súborov
msg-dirsize-no-folder = nie je tu priečinok, ktorý by sa dal zmerať

# Jobs: prompts, confirmations, progress, conflicts
prompt-copy-to = Kopírovať { $what } do:
prompt-move-to = Premenovať alebo presunúť { $what } do:
prompt-mkdir = Vytvoriť priečinok:
prompt-tab-name = Názov tejto karty:
prompt-bookmark-name = Názov tohto priečinka:
what-selected = vybraté položky ({ $count })
confirm-delete-title = Zmazať
confirm-delete-body =
    { $count ->
        [one] Chcete zmazať { $name } ({ $bytes })?
        [few] Chcete zmazať { $count } položky ({ $bytes })?
       *[other] Chcete zmazať { $count } položiek ({ $bytes })?
    }
prompt-link = Vytvoriť prepojenie na { $name } ako:
link-created = { $name } vytvorené
confirm-wipe-title = Vymazať
confirm-wipe-body =
    { $count ->
        [one] Prepísať a zmazať { $name } ({ $bytes })?
        [few] Prepísať a zmazať { $count } položky ({ $bytes })?
       *[other] Prepísať a zmazať { $count } položiek ({ $bytes })?
    }
confirm-wipe-honest = Bajty sa raz prepíšu a potom sa súbor odstráni. Na SSD, na súborovom systéme copy-on-write alebo pri snímkach môžu staršie kópie prežiť — a názov zostane v priečinku a v žurnáli.
job-copy = Kopírujem
job-move = Presúvam
job-delete = Mažem
job-wipe = Vymazávam
job-trash = Presúvam do Koša
job-replace = Nahrádzam
job-pack = Pakujem
job-rebuild = Zapisujem archív

# ── the attributes dialog (SPEC 10 §3.8) ───────────────────────────────────────────────
attr-title = Atribúty súboru
attr-count = vybraté položky: { $count }
attr-now = teraz { $text } ({ $octal })
attr-owner-read = Vlastník smie čítať
attr-owner-write = Vlastník smie zapisovať
attr-owner-exec = Vlastník smie spúšťať
attr-group-read = Skupina smie čítať
attr-group-write = Skupina smie zapisovať
attr-group-exec = Skupina smie spúšťať
attr-other-read = Ostatní smú čítať
attr-other-write = Ostatní smú zapisovať
attr-other-exec = Ostatní smú spúšťať
attr-setuid = Spúšťať ako vlastník (setuid)
attr-setgid = Spúšťať ako skupina (setgid)
attr-sticky = Zmazať smie len vlastník (sticky)
attr-readonly = Len na čítanie
attr-hidden = Skrytý
attr-system = Systémový
attr-archive = Archivovať
attr-temporary = Dočasný
attr-offline = Offline
attr-indexed = Indexovanie povolené
attr-compressed = komprimovaný
attr-encrypted = šifrovaný
attr-sparse = rozptýlený
attr-reparse = reparse point
attr-also = tiež { $what }
attr-mtime = Zmenený
attr-atime = Posledný prístup
attr-btime = Vytvorený
attr-ctime = Zmena záznamu
attr-btime-readonly = (čas vytvorenia sa dá nastaviť len vo Windows)
attr-utc = Časy v UTC
attr-owner = Vlastník (užívateľ:skupina)
attr-owner-is = vlastník { $who }
btn-attr-original = Originálny
btn-attr-current = Aktuálny
btn-attr-blank = Prázdny
attr-recursive = Aj všetko vnútri priečinkov
attr-bad-time = `{ $text }` nie je dátum a čas (RRRR-MM-DD HH:MM:SS)
attr-bad-owner = účet ani skupina `{ $text }` tu neexistuje
attr-in-archive = táto zmena zapíše archív nanovo, všetky ostatné položky zostanú, ako sú
job-attrs = Nastavujem atribúty

# ── the copy dialog (SPEC 03 §4) ───────────────────────────────────────────────────────
copy-title = Kopírovanie
copy-title-move = Premenovanie/Presunutie
copy-target = Kam
copy-conflict = Už existujúce súbory
copy-conflict-ask = opýtať sa
copy-conflict-overwrite = prepísať
copy-conflict-newer = prepísať, ak je zdroj novší
copy-conflict-skip = preskočiť
copy-conflict-rename = dať kópii voľný názov
copy-conflict-append = pripojiť na koniec
copy-only-newer = Len súbory novšie než tie v cieli
copy-filter = Len názvy, ktoré vyhovujú
copy-follow-links = Kopírovať obsah prepojení, nie prepojenia
copy-keep-times = Zachovať časy
copy-keep-mode = Zachovať prístupové práva
copy-verify = Každý súbor znova prečítať a porovnať
copy-several = cieľov: { $count }, jeden po druhom

# ── the terminal (SPEC 06 §4) ──────────────────────────────────────────────────────────
term-started = terminál, v ktorom beží { $shell }
term-hint = Ctrl+Alt+E: späť do panelov
term-ended = skončil — ľubovoľný kláves ho zatvorí
job-mkdir = Vytváram priečinok
job-planning = plánujem…
job-counters = súbory { $files } / { $filestotal } · { $bytes } / { $bytestotal }
job-rates = { $speed }/s · uplynulo { $elapsed } · zostáva { $eta }
job-rates-no-eta = { $speed }/s · uplynulo { $elapsed }
job-hint = Esc zruší
job-hint-queue = Esc zruší · B na pozadie
job-queued = { $kind } je vo fronte, nedokončených úloh pred ňou: { $ahead } · Alt+Shift+F5 ich vypíše
job-background = { $kind } pokračuje na pozadí · Alt+Shift+F5 vypíše úlohy
job-asks = { $kind } čaká na odpoveď · Alt+Shift+F5
job-paused = { $kind }: pozastavené
job-resumed = { $kind }: pokračuje
job-retry-nothing = Nie je čo opakovať
job-still-waiting = Táto úloha čaká, kým príde na rad
jobs-title = Úlohy (nedokončené: { $active })
jobs-hint = Enter zobraziť · Space pauza · Del zastaviť · F5 znova · Ctrl+↑↓ poradie · Esc zatvoriť
jobs-quit-title = Úlohy ešte bežia
jobs-quit-body = Ak sa program teraz ukončí, tieto úlohy sa zastavia:
job-state-running = beží
job-state-paused = pozastavená
job-state-asking = pýta sa
job-state-queued = čaká
job-state-held = podržaná
job-state-done = hotová
job-state-failed = chyby
job-state-cancelled = zrušená
job-done = { $kind }: súborov { $files }, { $bytes }
job-done-renamed = { $kind }: súborov presunutých bez kopírovania: { $files }
job-done-skipped = , preskočených: { $skipped }
job-done-verified = , overených: { $verified }
job-done-unverified = , nedalo sa tu znova prečítať: { $unverified }
job-done-errors = , chýb: { $errors }
job-cancelled = { $kind }: zrušené
job-errors-title = Chyby
conflict-title = Súbor už existuje
conflict-dst = existujúci  { $path }   { $bytes }
conflict-src = nový        { $path }   { $bytes }
conflict-newer = zdroj je novší
conflict-older = cieľ je novší
btn-overwrite = Prepísať
btn-overwrite-all = Všetky
btn-skip = Preskočiť
btn-skip-all = Preskočiť všetky
btn-retry = Znova
btn-move = Presunúť
btn-delete = Zmazať
btn-wipe = Vymazať
btn-copy = Kopírovať
btn-discard = Zahodiť
btn-kill = Ukončiť
btn-print = Tlač
btn-quit = Koniec
btn-play = Prehrať
btn-run = Spustiť
btn-accept = Prijať
btn-cancel = Storno
msg-nothing-to-do = nič nie je vybraté
cmd-complete-nothing = niet čím to doplniť
complete-several = takto sa začína názvov: { $count }
form-moving = šípky posúvajú dialóg · Enter alebo Esc ho tam nechá
form-moved = dialóg zostane tam, kam ste ho dali
msg-trash-refused = Kôš to neprijal — Shift+Del zmaže natrvalo.

# Checksums (SPEC 03 §9)
sum-title = Kontrolné súčty
sum-algorithm = Hash
sum-one-file = Jeden zoznam pre všetky
sum-name = Názov
sum-working = { $name }: { $done } z { $total }
sum-verifying = overujem podľa { $name }
sum-stopped = zastavené
sum-written = { $name } zapísaný
sum-no-name = zoznam potrebuje názov
sum-local-only = kontrolný súčet vie čítať len súbor na tomto počítači
print-title = Tlač
print-body = Vytlačiť {$what}?
print-sending = odosielanie na tlačiareň…
print-sent = odoslané na tlačiareň
print-failed = tlač zlyhala ({$line}): {$error}
print-local-only = vytlačiť sa dá len súbor na tomto počítači
print-no-command = chýba príkaz tlače: nastavte [print] command alebo nainštalujte lp či lpr
sum-all-match = všetkých { $count } súhlasí
sum-verify-title = Kontrolné súčty
sum-verify-summary = súhlasí: { $ok }, nesúhlasí: { $bad }
sum-differs = nesúhlasí
sum-missing = chýba
sum-unreadable = nedá sa prečítať

# Synchronising two directories (SPEC 03 §10)
sync-title = Synchronizácia
sync-between = { $left }  ↔  { $right }
sync-recursive = Aj všetko v podpriečinkoch
sync-both-ways = Oboma smermi, novší vyhráva
sync-delete-extra = Ponúknuť odstránenie toho, čo druhá strana nemá
sync-by-content = Porovnávať bajty, nie veľkosť a čas
sync-filter = Len názvy, ktoré vyhovujú
sync-walking = prezerám { $at } — zatiaľ { $pairs }
sync-planned = kopírovať: { $copies } ({ $bytes }), odstrániť: { $deletes }
sync-hint = Space ďalšia odpoveď · ← → smer · = nechať · F2 vykonať · Esc zrušiť
sync-hint-walking = Esc zastaví prechádzanie
sync-nothing-to-do = každý riadok hovorí nechať veci, ako sú
sync-local-only = synchronizácia vie čítať a zapisovať len na tomto počítači
sync-same-directory = oba panely sú v tom istom priečinku

# What to run for a kind of file (SPEC 06 §5)
assoc-none = nič tu nehovorí, ako otvoriť { $name }
usermenu-empty = Užívateľské menu ešte neexistuje: napíšte ho do { $path }
usermenu-last-gone = naposledy zvolený riadok v tomto menu už nie je
usermenu-bad-condition = Nezrozumiteľná podmienka — { $problem }
assoc-desktop = Odovzdať pracovnej ploche
assoc-open-with = Otvoriť { $name } pomocou
assoc-no-pager = žiadna asociácia ani $PAGER nehovorí, čím to čítať
assoc-no-editor = žiadna asociácia ani $VISUAL nehovorí, čím to upravovať

# Viewer (F3)
key-wrap = Zalom
key-hex = Hex
key-text = Text
key-dump = Dump
key-doc = Dok
key-search = Hľadať
key-code = Kód
key-follow = Sleduj
viewer-prompt-search = Hľadať ({ $mode }):
viewer-prompt-goto = Ísť na pozíciu alebo NN% :
viewer-not-found = `{ $query }` sa nenašlo
viewer-follow-on = sledujem koniec súboru
viewer-follow-off = sledovanie vypnuté
viewer-codepage = tabuľka znakov: { $name }
viewer-wrap-mode = { $mode }
viewer-no-undo = žiadna predchádzajúca pozícia
viewer-empty = (prázdny súbor)
viewer-not-markdown = nie je to súbor Markdown alebo je priveľký na vykreslenie
viewer-not-table = nie je to tabuľkový súbor alebo je priveľký na načítanie celý
viewer-base64-done = Dekódované: { $bytes }
viewer-not-base64 = Tento súbor nie je base64: { $why }
viewer-status = { $mode } { $codepage }  { $offset } / { $len }  { $percent }%

key-disasm = Disasm
viewer-disasm-failed = nedá sa disasemblovať { $name }: { $error }
viewer-decompile-running = { $tool } spracúva { $name }…
viewer-decompile-none = nič tu nevie dekompilovať { $format }; nainštalujte nástroj a pridajte pravidlo [[decompile]] do fcmd.toml
arc-needs-program = tento archív číta `{ $program }`, ktorý na tomto počítači nie je nainštalovaný
viewer-decompile-failed = { $tool }: { $error }
viewer-blame-derived = git blame patrí samotnému súboru: najprv sa k nemu vráťte
viewer-derived = { $producer } pre { $name }

# Editor (F4)
key-save = Uložiť
key-replace = Nahraď
editor-prompt-goto = Ísť na riadok alebo riadok:stĺpec :
editor-prompt-save-as = Uložiť súbor ako:
editor-saved = { $name }: uložené, bajtov: { $bytes }
editor-nothing-to-save = { $name } nemá neuložené zmeny
editor-not-found = `{ $query }` sa nenašlo
editor-replaced = nahradení: { $count }
editor-read-only = { $name } je len na čítanie
editor-readonly-on = len na čítanie: úpravy sa odmietajú
editor-readonly-off = režim len na čítanie vypnutý
editor-no-undo = niet čo vrátiť späť
editor-no-redo = niet čo zopakovať
editor-discard-title = Neuložené zmeny
editor-discard-body = { $name } má neuložené zmeny. Odísť a stratiť ich?
editor-changed-title = Zmenené na disku
editor-changed-body = { $name } sa od otvorenia na disku zmenil. Prepísať ho?
editor-reloaded = { $name } znova načítaný z disku
editor-failed = { $name }: { $error }

# netbox (SPEC 04 §6)
net-sites-title = Pripojenia
net-sites-hint = Enter pripojiť · n URL · Ins uložiť · F4 upraviť · d odpojiť · Esc zatvoriť
net-no-sites = Zatiaľ žiadne servery. Stlačte `n` a napíšte URL, alebo pridajte bloky [[site]] do konfigurácie.
net-not-a-stored-session = tento riadok je otvorené pripojenie, nie uložená relácia
net-site-forgotten = { $name } už nie je uložené
net-prompt-url = Pripojiť sa k:
net-prompt-password = Heslo pre { $user }@{ $host }:
net-prompt-passphrase = Heslo ku kľúču { $user }@{ $host }:
net-connecting = pripájam sa k { $host }…
net-connected = pripojené k { $host }
net-disconnected = { $host } odpojený
net-not-connected = { $host } nie je pripojený
net-local-panel = tento panel je na lokálnom súborovom systéme
net-nothing-pending = žiadne pripojenie na to nečaká
net-hostkey-title = Neznámy kľúč hostiteľa
net-hostkey-body = { $host } nie je v known_hosts. Jeho odtlačok je:
net-hostkey-question = Zapísať ho a pripojiť sa?
net-hostkey-changed-title = Kľúč hostiteľa sa ZMENIL
net-hostkey-changed-body = { $host } odpovedal kľúčom, ktorý nie je ten zapísaný.
net-hostkey-changed-where = Zapísaný je v { $path }, riadok { $line }.
net-hostkey-changed-what = Preinštalovaný server vyzerá presne ako niekto uprostred. Nič sa tu neprijíma: otvorte súbor, prečítajte riadok a odstráňte ho sami, ak viete, prečo sa zmenil.
btn-open-file = Otvoriť súbor
net-fetching = sťahujem { $name }…
net-viewing-copy = { $name }: lokálna kópia stiahnutá pre zobrazovač
net-too-big = { $name } je väčší než { $limit }: na stiahnutie zvýšte net.fetch_limit_mb
net-editing = { $name }: úprava lokálnej kópie; uloženie ju zapíše späť
net-writing-back = zapisujem { $name } späť…
net-written-back = { $name }: zapísané späť, bajtov: { $bytes }
net-write-back-failed = zápis späť zlyhal: { $error } — vaše úpravy sú v { $local }
net-changed-title = Zmenené na serveri
net-changed-body = { $name } sa od stiahnutia na serveri zmenil. Prepísať ho?
confirm-delete-permanent = Toto maže natrvalo: pre tieto položky Kôš nie je.
confirm-trash-title = Presunúť do Koša
confirm-trash-body =
    { $count ->
        [one] Chcete presunúť { $name } ({ $bytes }) do Koša?
        [few] Chcete presunúť { $count } položky ({ $bytes }) do Koša?
       *[other] Chcete presunúť { $count } položiek ({ $bytes }) do Koša?
    }
confirm-trash-where = Pôjdu tam, kde pracovná plocha drží zmazané súbory, a dajú sa odtiaľ obnoviť.

# ---------------------------------------------------------------- menu bar (SPEC 10)
menu-left = &Ľavý
menu-right = &Pravý
menu-files = &Súbory
menu-commands = Prí&kazy
menu-options = &Nastavenia
menu-brief = &Stručný
menu-medium = S&tredný
menu-full = Ú&plný
menu-detailed = Pod&robný
menu-owners = Vlastníci s&úboru
menu-links = Prepo&jenia súboru
menu-alt-full = Úplný, veľkosti &ako celé čísla
menu-descriptions = S p&opismi
menu-long-descriptions = S &dlhými popismi
menu-wide = Š&iroký
menu-info-panel = Panel In&fo
menu-quick-view = R&ýchle info
menu-tree-panel = Pa&nel Strom
menu-tab-new = Nová &karta
menu-tab-close = Zavrieť kartu
menu-tab-next = Ďa&lšia karta
menu-tab-prev = Predchádzajú&ca karta
menu-tab-move = Presunúť kartu do druhého pan&ela
menu-tab-lock = &Uzamknúť kartu na jej priečinok
menu-tab-name = Pomenovať túto kartu
menu-tab-list = Všetky karty
menu-sort-modes = Triediť pod&ľa
menu-hidden = Skryté a systémové súbory
menu-filter = Filter súboro&v
menu-filter-sets = Pomenované filtre
menu-panel-onoff = Panel &Zap/Vyp
menu-reread = O&bnoviť
menu-change-drive = Z&meniť jednotku
menu-view = &Zobraziť
menu-edit = &Upraviť
menu-edit-new = Upraviť &nový súbor
menu-copy = &Kopírovať
menu-rename = &Premenovať/Presunúť
menu-link = Vytvoriť prepo&jenie
menu-multirename = Premenovať podľa vzoru…
menu-multirename-undo = Vrátiť posledné hromadné premenovanie
menu-mkdir = &Vytvoriť priečinok
menu-delete = Z&mazať
menu-wipe = V&ymazať
menu-checksum = Kontrolné s&účty…
menu-checksum-verify = Overiť podľa zoznamu
menu-split = Rozdeliť súbor…
menu-combine = Spojiť súbory
menu-delete-permanent = Zmazať natrva&lo
menu-archive-add = Pridať do &archívu
menu-archive-extract = &Rozbaliť súbory
menu-archive-commands = Príkazy pakova&ča
menu-archive-test = Otestovať archív
menu-attributes = A&tribúty súboru
menu-apply-command = Pou&žiť príkaz
menu-replace-in-files = Nahradiť v súboroch…
menu-select-group = Označiť &skupinu
menu-select-ext = Označiť pod&ľa tejto prípony
menu-unselect-ext = Odznačiť podľa t&ejto prípony
menu-unselect-group = O&dznačiť skupinu
menu-invert-selection = &Invertovať výber
menu-select-all = Označiť všetko
menu-restore-selection = O&bnoviť výber
menu-dirsize = Veľk&osť priečinka
menu-find-file = Ná&jsť súbor
menu-history = &História príkazov
menu-view-history = História &zobrazení a úprav
menu-folders-history = História &priečinkov
menu-usage = Využitie dis&ku
menu-jobs = Úl&ohy
menu-branch = Zobrazenie vetvy
menu-bookmarks = Skratky priečinkov
menu-swap-panels = Vymeniť panel&y
menu-sync-panel = Zosúladiť druhý pa&nel
menu-panels-onoff = Pane&ly Zap/Vyp
menu-compare-folders = Po&rovnať priečinky
menu-sync-folders = &Synchronizovať priečinky
menu-compare-contents = Porovnať podľa o&bsahu
menu-panelize = Výstup príkazu do panela
menu-view-filtered = Zobraziť cez príkaz
menu-learn-keys = Naučiť klávesy
menu-tunnel = Otvoriť &tunel…
menu-tunnels = Tun&ely…
tunnel-title = Tunel: [lokálny:]hostiteľ:port
tunnel-list-title = Tunely
tunnel-none = žiadny tunel nie je otvorený
tunnel-open = tunel otvorený: { $what }
tunnel-closed = tunel zatvorený
tunnel-failed = { $what }: { $why }
tunnel-needs-connection = tunel vedie cez pripojenie: najprv ho otvorte pomocou Alt+N
menu-connections = Pripojenia
menu-disconnect = Odpojiť
menu-user-menu = &Užívateľské menu
menu-associations = Asociá&cie súborov
menu-folder-shortcuts = Skr&atky priečinkov
menu-plugin-commands = Príkazy mo&dulov
menu-screens-list = Zoznam ok&ien
menu-macro-record = Zaznamenať makro
menu-macro-list = Makrá
menu-task-list = Zoznam &úloh
menu-system-settings = S&ystémové nastavenia
menu-panel-settings = Nastavenia &panelov
menu-interface-settings = Nastavenia ro&zhrania
menu-languages = &Jazyky
menu-themes = &Témy
menu-confirmations = P&otvrdzovanie
menu-viewer-settings = Nastavenia zob&razovača
menu-editor-settings = Nastavenia &editora
menu-codepages = Ta&buľky znakov
menu-sort-groups = &Skupiny triedenia
menu-mask-groups = Skupiny &masiek súborov
menu-settings-all = Všetky n&astavenia
menu-highlighting = Z&výrazňovanie súborov
menu-plugins-config = Nastavenia mo&dulov
menu-help = Pomocník
menu-save-setup = &Uložiť nastavenia
menu-quit = Ko&niec
opt-natural-sort = Považovať cifry za čísla
opt-allow-reverse = Povoliť obrátené triedenie
opt-select-folders = Masky označujú aj priečinky
cmd-no-network-name = Cesta na tomto disku nemá sieťový názov
opt-trash = Mazať do Koša
opt-system-copy = Použiť systémovú kopírovaciu rutinu
opt-save-history = Ukladať históriu medzi reláciami
opt-history-lifetime = Životnosť histórie v dňoch
opt-forever = navždy
opt-fetch-limit = Limit veľkosti vzdialeného súboru
opt-keepalive = SSH keep-alive
opt-off = vypnuté
opt-show-hidden = Zobraziť skryté a systémové súbory
opt-column-titles = Zobrazovať názvy stĺpcov
opt-status-line = Zobrazovať stavový riadok
opt-scrollbar = Zobrazovať posuvník
opt-free-space = Zobrazovať voľné miesto
opt-auto-refresh = Automaticky obnovovať panel
opt-dirs-first = Najprv priečinky
opt-selected-first = Najprv vybraté súbory
opt-reverse = Obrátené poradie
opt-view = Zobrazenie panela
opt-sort = Triediť podľa
opt-quick-search = Rýchle hľadanie
opt-quick-alt = s Alt
opt-quick-plain = písmenami
opt-key-bar = Zobrazovať tlačidlá funkčných klávesov
opt-status-bar = Zobrazovať stavový pruh
opt-syntax = Farbiť jazyky, ktoré poznáme
opt-markdown = Markdown vykreslený na čítanie
opt-table = Otvárať .csv alebo .jsonl ako tabuľku
opt-keymap = Mapa klávesov
opt-clock = Hodiny v rohu
opt-mouse = Myš
opt-clipboard = Nechať terminál prenášať schránku (OSC 52)
opt-command-screen = Zobrazovať výstup príkazu počas behu
opt-menu-always = Vždy zobrazovať pruh menu
opt-save-exit = Pri ukončení uložiť nastavenia
opt-confirm-delete = Zmazanie
opt-confirm-overwrite = Prepísanie
opt-confirm-exit = Ukončenie
opt-confirm-editor = Odchod z editora s neuloženými zmenami
opt-applied = nastavenia použité
opt-saved = uložené do { $path }
opt-set-title = { $key }
opt-set = { $key } = { $value }
opt-not-a-toggle = Toto nie je nastavenie áno/nie: Enter zapíše hodnotu
opt-settings-hint = ↑↓ výber · píšte na zúženie · Enter alebo F4 hodnota · Space prepnúť · Del predvolené · Esc zatvoriť
opt-language-set = jazyk: { $name }
sort-name = Názov
sort-ext = Prípona
sort-size = Veľkosť
sort-mtime = Čas zápisu
sort-btime = Čas vytvorenia
sort-atime = Čas prístupu
sort-descr = Popis
sort-owner = Vlastník
sort-git = Stav Git
key-sort-descr = Popis
key-sort-owner = Vlastn
sort-none = Netriediť
sort-set = triedenie: { $name }
btn-ok = OK
pick-hint = ↑↓ výber · píšte na filtrovanie · Enter použiť · Esc zatvoriť
pick-empty = (nie je z čoho vybrať)
# ------------------------------------------------- the three histories (SPEC 06 §6)
hist-pinned = { $line } je pripnutý · teraz ho nič nevytlačí
hist-unpinned = { $line } už nie je pripnutý
hist-is-pinned = { $line } je pripnutý · Ins ho najprv odopne
hist-clear-body = Vyprázdniť túto históriu od všetkého, čo nie je pripnuté?
hist-cleared = história je prázdna až na to, čo je pripnuté
history-hint = ↑↓ výber · Enter použiť · Ins pripnúť · Shift+Del jeden · Del všetko · Esc zatvoriť


# ------------------------------------------------- session editor (SPEC 10 §3.5)
site-new = Nové pripojenie
site-edit = Upraviť pripojenie
site-name = Názov
site-host = Hostiteľ
site-port = Port
site-user = Užívateľ
link-title = Vytvoriť prepojenie
link-name = Prepojenie
link-kind = Druh
link-kind-relative = symbolické, relatívne — prežije presun stromu
link-kind-absolute = symbolické, absolútne — prežije presun prepojenia
link-kind-hard = pevné — druhý názov toho istého súboru
menu-colors = &Farby…
palette-title = Farby témy { $theme }
palette-hint = Enter farba · F2 uložiť · Esc zatvoriť
palette-ask = Farba pre { $key }
palette-bad-color = { $value } nie je farba (názov, 0–255 alebo #rrggbb)
palette-saved = farby boli zapísané do témy { $theme }
palette-unsaved = farby sa zmenili len na obrazovke — F2 ich zapíše
opt-viewer-status-line = Zobrazovať horný riadok (názov a pozícia)
opt-viewer-scrollbar = Zobrazovať posuvník
opt-viewer-key-bar = Zobrazovať tlačidlá funkčných klávesov
opt-viewer-wrap = Dlhé riadky
opt-viewer-wrap-off = nezalamovať
opt-viewer-wrap-char = zalamovať na okraji
opt-viewer-wrap-word = zalamovať po slovách
opt-viewer-tab-width = Šírka tabulátora
opt-viewer-remember = Pamätať si pozíciu a tabuľku znakov každého súboru
opt-editor-tabs = Tabulátory
opt-editor-tabs-keep = nechať, ako sú
opt-editor-tabs-typed = rozvinúť to, čo sa píše
opt-editor-tabs-all = rozvinúť všetky pri otvorení súboru
opt-editor-tab-width = Šírka tabulátora
opt-editor-auto-indent = Zachovať odsadenie riadka nad
opt-editor-whitespace = Zobrazovať medzery a tabulátory
opt-editor-cursor-after = Kurzor skončí za textom, ktorý hľadanie našlo
opt-editor-beyond-eol = Kurzor môže ísť za koniec riadka
opt-editor-trim-trailing = Pri ukladaní odstrániť medzery na konci riadkov
editor-not-a-character = tento kláves nemá znak, ktorý by sa dal vložiť
paste-one-line =
    { $count ->
        [one] vložený len prvý riadok · jeden ďalší riadok vynechaný
        [few] vložený len prvý riadok · { $count } ďalšie riadky vynechané
       *[other] vložený len prvý riadok · { $count } ďalších riadkov vynechaných
    }
clip-names =
    { $count ->
        [one] jeden názov v schránke
        [few] { $count } názvy v schránke
       *[other] { $count } názvov v schránke
    }
clip-paths =
    { $count ->
        [one] jedna cesta v schránke
        [few] { $count } cesty v schránke
       *[other] { $count } ciest v schránke
    }
clip-line = príkazový riadok je v schránke
clip-selection = označený text je v schránke
clip-off = schránka je vypnutá — `ui.clipboard` v nastaveniach
history-gone = { $name } už neexistuje
hl-new = Nová farebná skupina
hl-edit = Farebná skupina
# ------------------------------------------------- the diff screen (SPEC 05 §3)
diff-title = Rozdiely
menu-diff = Zobraziť rozdiely súboro&v
diff-needs-two = Dajte kurzor na súbor v každom paneli
diff-not-here = { $name } nie je súbor na tomto disku
diff-too-big = { $name } je väčší než { $most }
diff-same = Oba súbory sú rovnaké
diff-where = úsek { $at } z { $count } · riadky -{ $removed } +{ $added }
diff-no-next = Posledný rozdiel
diff-no-previous = Prvý rozdiel
diff-hint = Alt+↑↓ úseky · Tab strana · F5 prevziať · F2 zapísať · U zjednotený · Shift+F2 patch · W C E ignorovať · Esc
diff-hint-git = Alt+↑↓ úseky · S pripraviť úsek · Tab strana · F5 prevziať · F2 zapísať · Esc zatvoriť
diff-hint-git-staged = Alt+↑↓ úseky · S zrušiť prípravu úseku · Tab strana · F5 prevziať · F2 zapísať · Esc zatvoriť
diff-stage-edited = diff má nezapísané zlúčenia: najprv F2, potom S
diff-merged-left = Ľavý súbor prevzal tento rozdiel
diff-merged-right = Pravý súbor prevzal tento rozdiel
diff-no-hunk-here = Navrchu obrazovky nie je rozdiel — Alt+↓ tam jeden dá
diff-nothing-to-write = Nič nebolo zlúčené
diff-written = zapísané: { $count }
diff-unwritten = Zlúčenie nebolo zapísané — F2 ho zapíše, ďalší Esc ho zahodí
diff-ignore-space-on = Medzery sa ignorujú
diff-ignore-space-off = Medzery sa znova počítajú
diff-ignore-case-on = Veľkosť písmen sa ignoruje
diff-ignore-case-off = Veľkosť písmen sa znova počíta
diff-ignore-eol-on = Konce riadkov sa ignorujú
diff-ignore-eol-off = Konce riadkov sa znova počítajú

grab-hint = ↑↓←→ pohyb · Shift+↑↓←→ označiť · Ctrl+A všetko · Enter kopírovať · Esc zatvoriť
grab-copied = skopírovaných riadkov obrazovky: { $rows }

# ------------------------------------------------- the code pages (SPEC 05 §1.12)
cp-title = Tabuľky znakov
cp-name-title = Ako nazvať túto tabuľku znakov
cp-set = Čítam ako { $name }
menu-panel-names = Tabuľka znakov názvov…
names-title = Čítať názvy ako
names-system = Ako ich dáva systém
names-set = Názvy sa čítajú ako { $name }
names-system-set = Názvy tak, ako ich dáva systém
names-not-here = Názov v tomto systéme nie sú bajty, takže sa nedá čítať inak
cp-marked = F8 teraz prechádza aj cez { $name }
cp-unmarked = F8 už neprechádza cez { $name }
cp-hint = ↑↓ výber · Enter čítať tak · Ins označiť pre F8 · F4 premenovať · Esc zatvoriť

# ------------------------------------------------- the named mask groups (SPEC 02 §5b)
mg-title = Skupina masiek
mg-name = Názov
mg-mask = Masky
mg-hint = Ins pridá skupinu, Del ju odstráni, F4 upraví · `·` označuje tri vstavané
mg-saved = <{ $name }> platí; Uložiť nastavenia ju zapíše
mg-removed = <{ $name }> je preč
mg-bad = <{ $name }>: { $problem }
mg-needs-name = názov bez medzier, čiarok a lomených zátvoriek
mg-needs-mask = skupina, ktorá nič nezastupuje, ničomu nevyhovie
mg-built-in = <{ $name }> je jedna z troch vstavaných · F4 jej dá iné masky

sg-title = Skupina triedenia
sg-name = Názov
sg-mask = Masky
sg-place = Kde
sg-place-front = pred ostatnými
sg-place-back = za ostatnými
sg-hint = Ins pridá skupinu, Del ju odstráni, F4 upraví, Ctrl+↑↓ presunie
sg-saved = Skupina platí; Uložiť nastavenia ju zapíše
sg-removed = { $name } je preč
sg-bad = { $name }: { $problem }
sg-needs-mask = skupina bez masky by nič neobsahovala
hl-name = Názov
hl-mask = Maska alebo masky
hl-kind = Platí pre
hl-kind-both = súbory a priečinky
hl-kind-file = len súbory
hl-kind-dir = len priečinky
hl-size = Veľkosť (1M-10M, -4K, 1G-)
hl-age = Vek v dňoch (<=1, >=30, 7-30)
hl-date-field = Vek sa týka
hl-attrs = Atribúty (h r x s b, - znamená nesmie mať)
hl-fg = Farba textu (#d7875f, red, file.archive)
hl-bg = Pozadie, zapísané rovnako
hl-hint = ↑↓ výber · Enter/F4 upraviť · Ins nová · Del odstrániť · Ctrl+↑↓ presunúť · Esc zatvoriť
hl-saved = skupina platí · Nastavenia → Uložiť nastavenia ju zapíše do súboru
hl-removed = { $name } je preč · Nastavenia → Uložiť nastavenia to zapíše do súboru
hl-bad = { $name }: { $problem }
filter-off = Bez filtra: všetky názvy
filter-hint = ↑↓ výber · Enter použiť · F4 upraviť · Ins nový · Del odstrániť · Esc zatvoriť
filter-new = Nový filter
filter-edit = Filter
filter-name-hint = s názvom zostane v menu filtrov; bez názvu sa použije raz
filter-applied = filter zapnutý: { $name }
filter-cleared = filter vypnutý
filter-saved = { $name } je na paneli aj v menu filtrov · Nastavenia → Uložiť nastavenia ho zapíše do súboru
filter-removed = { $name } je preč · Nastavenia → Uložiť nastavenia to zapíše do súboru
editor-no-block = nič nie je označené
editor-unique = odstránených opakovaných riadkov: { $count }
editor-no-bracket = tu nie je zátvorka alebo k nej nič nepatrí
editor-tabs-expanded = tabulátory boli rozvinuté na { $lines } riadkoch — uloženie to zapíše
site-protocol = Protokol
site-protocol-sftp = SFTP (cez SSH)
site-protocol-ftp = FTP
site-protocol-ftps = FTP s TLS
site-protocol-dav = WebDAV
site-protocol-davs = WebDAV cez HTTPS
site-auth = Overenie
site-auth-agent = agent SSH
site-auth-key = súbor s kľúčom
site-auth-ask = pýtať si heslo
site-key-file = Súbor s kľúčom
site-start-dir = Vzdialený priečinok
site-group = Skupina
site-keepalive = Keep-alive (s)
site-compress = Kompresia
site-page-session = Relácia
site-page-environment = Prostredie
site-page-directories = Priečinky
site-page-sftp = SFTP
site-password-note = Heslo sa neukladá: vyžiada sa pri pripájaní.
site-env-eol = Prekladať konce riadkov (zatiaľ nie)
site-env-utf8 = Vynútiť názvy v UTF-8 (zatiaľ nie)
site-jump = Skokový hostiteľ
site-jump-note = Dosiahnuté cez tohto hostiteľa: host, user@host alebo user@host:port. Prázdne znamená priamo na server.
site-sftp-ciphers = Vybrať šifry (zatiaľ nie)
btn-connect = Pripojiť
site-needs-host = pripojenie potrebuje hostiteľa
site-needs-key = vyberte súbor s kľúčom alebo iný spôsob overenia
site-bad-port = `{ $port }` nie je číslo
site-stored = pripojenie uložené: { $name } (Shift+F9 ho zachová)
# ------------------------------------------------- structure pane (SPEC 05 §1.8)
key-struct = Štrukt
key-markdown = MD
key-table = Tabuľ
key-filter = Filter
key-decompile = Dekomp
key-next = Nasl.
key-prev = Predch
key-codepages = TabZn
key-goto = Ísť na
key-to-panel = Panel
key-blame = Blame
viewer-structure = Štruktúra
viewer-structure-hint = ↑↓ riadok · Enter ísť na · Tab obsah · Shift+F11 zatvoriť
viewer-structure-hint-tab = Tab presunie klávesy sem
viewer-no-structure = niet čo dekódovať: tento formát panel štruktúry nepozná
viewer-no-target = tento riadok neukazuje nikam do súboru
viewer-form-opened = Formulár ako text, ktorý preň zobrazuje Delphi. F4 sa vráti k bajtom.
viewer-metadata-opened = Celý text, upravený na čítanie. F4 sa vráti k bajtom.
viewer-went-to = prechod na { $offset }
viewer-in-panel = panel je na { $name }
viewer-not-in-panel = { $name } nie je v tomto paneli
viewer-no-more-files = v paneli nie sú ďalšie súbory
editor-unsaved-for-viewer = najprv uložte: zobrazovač číta súbor z disku
# ------------------------------------------------- command line (SPEC 06 §7)
cmd-running = beží: { $command }
prompt-panelize = Príkaz, ktorého výstup pomenúva súbory
prompt-viewer-filter = Príkaz, ktorého výstup sa číta namiesto súboru
learn-title = Ktoré klávesy tento terminál posiela
learn-hint = Stláčajte klávesy. Rozsvietená bunka dorazila; dvakrát Esc ukončí.
learn-last = posledný: { $key } — { $command }
learn-unbound = nie je k ničomu priradený
recode-title = Zapísať znova ako
recode-what = súborov: { $count }
recode-from = Čo sú teraz
recode-to = Zapísať ich ako
recode-auto = zistiť pre každý súbor
recode-eol = Konce riadkov
recode-eol-keep = bez zmeny
recode-working = zapisujem znova súbory: { $count }
recode-done = zapísaných: { $changed }, už v poriadku: { $same }, odmietnutých: { $failed }
menu-recode = Zapísať znova ako…
panelize-title = { $command }
panelize-done = súborov: { $count }, preskočených riadkov: { $dropped }
panelize-nothing = { $command }: žiadny riadok nepomenoval existujúci súbor
cmd-interrupted = príkazu bolo poslané prerušenie
cmd-done = { $command } skončil
cmd-failed = { $command } skončil s kódom { $code }
cmd-busy = príkaz už beží
prompt-apply = Príkaz, ktorý sa spustí pre každý z { $count } súborov:
cmd-apply-no-ask = príkaz, ktorý si pýta hodnotu, sa nedá použiť súbor po súbore
cmd-apply-progress = { $command }  ({ $done } z { $total })
cmd-apply-stopped = zastavené po { $done } z { $total }: { $command } zlyhal
cmd-not-remote = príkaz potrebuje lokálny priečinok; tento panel je na serveri
cmd-no-history = história príkazov je prázdna
cmd-no-previous = niet sa kam vrátiť
cmd-no-such-dir = { $path }: taký priečinok neexistuje
cmd-no-such-path = { $path }: taký súbor ani priečinok neexistuje
cmd-no-selection = nič nie je vybraté
cmd-condition-false = nespustené: { $command }
cmd-screen-cleared = obrazovka vymazaná
cmd-stack-empty = zásobník priečinkov je prázdny
cmd-stack-cleared = zásobník priečinkov vymazaný
cmd-pushed = priečinkov v zásobníku: { $count }
cmd-environment = prostredie
cmd-set = { $name } = { $value }
cmd-not-set = { $name } nie je nastavená
user-screen-title = Výstup — riadkov: { $lines }, od riadka { $at }
user-screen-empty = (zatiaľ sa nič nespustilo)
viewer-no-branch = tento riadok nemá adresu, na ktorú sa dá skočiť
viewer-no-back = niet sa k čomu vrátiť

# ------------------------------------------------- find files (SPEC 06 §1)
folders-title = Hľadať priečinok
folders-reading = Čítam priečinky…
folders-read = priečinkov: { $count }
folders-capped = priečinkov: { $count }, po limit
folders-local-only = zoznam priečinkov patrí tomuto počítaču; server má svoju históriu a skratky
find-title = Hľadať súbor
find-mask = Maska alebo masky súborov
find-size = Veľkosť (100M, 1M-1G, -4K)
find-text = Obsahujúci text
find-case = Rozlišovať malé a VEĽKÉ písmená
find-words = Len celé slová
find-hex = Text vyššie je hexadecimálny vzor
find-not = Súbory, ktoré ho NEOBSAHUJÚ
find-scope = Hľadať
find-scope-panel = len v tomto priečinku
find-scope-tree = od tohto priečinka
find-scope-selection = vo vybratých priečinkoch
find-scope-root = od koreňa tejto jednotky
find-scope-other = od druhého panela
find-links = Hľadať v symbolických prepojeniach
find-dirs = Hľadať aj priečinky
find-archives = Hľadať v archívoch
find-results = Nájdených { $count } · prezretých súborov { $files } a priečinkov { $dirs }
find-done = nájdených { $count }, súborov { $files } · { $elapsed }
find-none = nič sa nenašlo
find-stopped = zastavené po { $files } súboroch
find-hint = ↑↓ výber · Enter ísť na · F3 zobraziť · F4 upraviť · F5 do panela · Esc stop
find-hint-done = ↑↓ výber · Enter ísť na · F3 zobraziť · F4 upraviť · F5 do panela · Esc zatvoriť
find-panelised = výsledkov v paneli: { $count }
find-needs-mask = treba masku: `*` znamená každý súbor
find-bad-size = `{ $text }` nie je veľkosť — { $error }

# ------------------------------------------------- replace across files
replace-title = Nahradiť v súboroch
replace-needle = Hľadať
replace-with = Nahradiť čím
replace-case = Rozlišovať malé a VEĽKÉ písmená
replace-words = Len celé slová
replace-pattern = Text vyššie je regulárny výraz
replace-hex = Oboje sú hexadecimálne bajty
replace-scope = vybratých súborov: { $count }
replace-needs-needle = nie je čo hľadať
replace-confirm-title = Nahradiť
replace-confirm-body = Zmenilo by sa výskytov: { $count }, v súboroch: { $files }.
replace-nothing = nie je čo nahradiť: žiadny súbor by sa nezmenil
replace-done = nahradených výskytov: { $count }, v súboroch: { $files }

# ── archives (SPEC 04 §4) ───────────────────────────────────────────────────────────────
arc-writing-back = Zapisujem { $name } späť do archívu
arc-move-out = na vybratie niečoho z archívu to skopírujte klávesom F5
arc-add-files-only = po jednom sa pridávajú len súbory; Shift+F1 pridá celý priečinok

rename-title = Hromadné premenovanie
rename-name = Názov
rename-ext = Prípona
rename-search = Hľadať
rename-replace = Nahradiť čím
rename-regex = Regulárny výraz ($1 v náhrade)
rename-case = Písmená
rename-case-same = ako sú
rename-case-lower = malé písmená
rename-case-upper = VEĽKÉ PÍSMENÁ
rename-case-title = Každé Slovo Veľkým
rename-case-sentence = Len prvé písmeno
rename-counter = Počítadlo: začiatok, krok, cifry
rename-fields = [N] názov  [E] prípona  [N1-3] časť  [C] počítadlo  [d] [t] dátum, čas  [P] priečinok
rename-not-in-archive = viac súborov sa premenúva v priečinku, nie v archíve: F6 tam premenuje jednu položku
rename-bad-counter = počítadlo sú tri celé čísla, začiatok, krok, cifry: { $counter } to nie je
rename-cannot = nedá sa premenovať: { $why }
rename-problem-empty = nezostal žiadny názov
rename-problem-invalid = taký názov súbor mať nemôže
rename-problem-twice = tento názov dostane aj iný súbor
rename-problem-exists = súbor s týmto názvom už existuje
rename-more = … a ďalších { $count }
rename-blocked =
    { $count ->
        [one] jeden súbor nemôže dostať nový názov: kým nemôže, nič sa nepremenuje
        [few] { $count } súbory nemôžu dostať nové názvy: kým nemôžu, nič sa nepremenuje
       *[other] { $count } súborov nemôže dostať nové názvy: kým nemôžu, nič sa nepremenuje
    }
rename-summary = zmení sa { $count } z { $total }
rename-nothing = žiadny názov sa nemení
rename-nothing-to-undo = nie je hromadné premenovanie, ktoré by sa dalo vrátiť
rename-done =
    { $count ->
        [one] jeden súbor premenovaný · vrátite to z menu Súbory
        [few] { $count } súbory premenované · vrátite to z menu Súbory
       *[other] { $count } súborov premenovaných · vrátite to z menu Súbory
    }
rename-undone =
    { $count ->
        [one] jeden názov vrátený
        [few] { $count } názvy vrátené
       *[other] { $count } názvov vrátených
    }
pack-title = Pridať do archívu
pack-name = Archív
pack-format = Formát
pack-level = Kompresia
pack-solid = Súvislé bloky (7z: menší, čítanie jedného súboru pomalšie)
pack-delete-after = Po archivácii súbory zmazať
pack-level-normal = normálna
pack-level-fast = rýchla
pack-level-best = najlepšia
pack-scope = vybratých súborov: { $count }
pack-needs-name = archív potrebuje názov
pack-format-read-only = { $format } sa tu číta, nezapisuje: zvoľte iný formát alebo iný názov
pack-single-one-file = { $format } obsahuje jeden súbor: vyberte jeden, alebo zvoľte { $tar } pre priečinok či viac súborov
unpack-title = Rozbaliť z archívu
unpack-to = Rozbaliť do
unpack-scope = z { $name }
unpack-not-an-archive = nie je to archív, ktorý vieme čítať

# ── the special panel kinds (SPEC 02 §7) ────────────────────────────────────────────────
panel-quick-view = Rýchle info
panel-info = Informácie
panel-tree = Strom
compare-done = líši sa { $left } vľavo a { $right } vpravo
cmp-subdirs = Ísť do podpriečinkov
cmp-depth = Ako hlboko ísť (0 = až na dno)
cmp-trees-title = Čo sa líši
cmp-trees-working = porovnávam stromy: súborov { $seen }, prečítaných { $read }
cmp-trees-done = líši sa { $differ }, len vľavo { $left }, len vpravo { $right }, z { $pairs } dvojíc
cmp-trees-same = oba stromy sú rovnaké (dvojíc { $pairs }, overených { $same })
menu-duplicates = Nájsť duplikáty
dup-title = Duplicitné súbory
dup-working = hľadám duplikáty: súborov { $seen }, prečítaných { $hashed }
dup-done = skupín kópií { $groups }, súborov { $files }, zbytočne zabraté { $wasted }
dup-none = nič tu nie je kópiou ničoho iného (súborov: { $seen })
dup-stopped = zastavené po { $seen } súboroch
dup-partial = zastavené predčasne
cmp-title = Porovnať
cmp-case = Pri názvoch rozlišovať malé a VEĽKÉ písmená
cmp-size = Iná veľkosť je rozdiel
cmp-time = Iný čas je rozdiel
cmp-seconds = Časy vzdialené o toľkoto sekúnd sú ten istý okamih
cmp-contents = Čítať bajty
cmp-when-time = Čítať ich len tam, kde sa líšia časy
cmp-fragment = Čítať len prvých toľkoto KiB (0 = všetko)
cmp-ignore-eol = Samotné konce riadkov nie sú rozdiel
cmp-selected = Len to, čo je vybraté
cmp-stop = Zastaviť pri prvom rozdiele
cmp-newer = Označiť len novšiu kópiu
menu-compare-setup = Porovnať s &možnosťami
compare-contents-eol-ignored = { $count } sa líši len koncami riadkov, ignorované
compare-contents-trusted = { $count } považovaných za rovnaké bez čítania
compare-contents-working = Porovnávam obsah: { $done } z { $total }
compare-contents-done = líši sa { $differ } z { $pairs } dvojíc
compare-contents-same = všetkých { $pairs } dvojíc je zhodných
compare-contents-none = žiadny súbor nemá rovnaký názov na oboch stranách
compare-contents-local = porovnanie podľa obsahu potrebuje oba panely na lokálnom súborovom systéme
compare-contents-eol = { $count } len v koncoch riadkov
compare-contents-only = { $count } len na jednej strane
compare-contents-unread = { $count } sa nedalo prečítať
compare-contents-stopped = porovnávanie zastavené
quick-view-directory = { $name } je priečinok — F3 ho zmeria
quick-view-not-here = bajty tu nie sú: F3 ich najprv prinesie
info-path = Cesta
info-kind = Druh
info-kind-local = priečinok na tomto počítači
info-kind-remote = na { $host }
info-kind-archive = vnútri { $name }
info-kind-list = zoznam súborov
info-device = Zariadenie
info-folder = Priečinok
info-device-value = { $total }, voľné { $free }, klastre { $cluster }
info-slack = Mŕtve miesto
info-filesystem = Súborový systém
info-mount = Pripojené
info-git = Repozitár
info-git-clean = zhodné s indexom
info-git-differing = { $count } z { $tracked } sa líši od indexu
info-git-uncounted = nespočítané
info-git-merge = prebieha merge
info-git-rebase = prebieha rebase
info-git-cherry-pick = prebieha cherry-pick
info-git-revert = prebieha revert
info-git-bisect = prebieha bisect
info-memory = Pamäť
info-memory-value = voľné { $free } z { $total }
info-memory-total = { $total }
info-contents = Obsahuje
info-contents-value = súbory: { $files }, priečinky: { $dirs }, { $bytes }
info-selected = Vybraté
info-selected-value = súbory: { $count }, { $bytes }
info-cursor = Kurzor
info-size = Veľkosť
info-modified = Zmenené
info-description = Popis
key-mark = Označ
key-asm = Asm
key-block = Blok
key-screens = Okná
key-site-edit = UprRel
key-site-new = NováRel
key-site-forget = ZmažRel
key-ai-edit = Uprav
key-ai-new = Nová
key-ai-remove = Odstrán
key-ai-forget = Zabudni
key-ai-threads = Histór
hex-needs-a-file = tento buffer nemá súbor, z ktorého by sa čítali bajty
hex-save-first = najprv uložte: dva editory nemôžu držať neuložené zmeny toho druhého

# ── the hex editor's block operations (SPEC 05 §2.6) ────────────────────────────────────
hex-block-title = Blok
hex-op-write = Zapísať { $size } do súboru
hex-op-read = Načítať súbor na { $at }
hex-op-insert = Vložiť nulové bajty na { $at }
hex-op-copy = Skopírovať bajty odinakiaľ na { $at }
hex-op-move = Presunúť bajty odinakiaľ na { $at }
hex-op-fill = Vyplniť { $size } bajtom
hex-op-xor = XOR { $size } s kľúčom
hex-op-delete = Zmazať { $size } — z { $was } bude { $becomes }
hex-write-title = Zapísať blok do
hex-read-title = Ktorý súbor načítať
hex-fill-title = Ktorým bajtom vyplniť (hex)
hex-xor-title = XOR s ktorým kľúčom (hex)
hex-insert-title = Koľko nulových bajtov
hex-copy-title = Odkiaľ kopírovať — pozícia a dĺžka (hex)
hex-move-title = Odkiaľ presunúť — pozícia a dĺžka (hex)
hex-not-hex = to nie je postupnosť hexadecimálnych číslic
hex-not-a-count = to nie je počet bajtov
hex-not-a-range = to nie je pozícia a dĺžka v hex
hex-range-outside = tento rozsah je mimo súboru ({ $size })
hex-move-into-itself = blok sa nedá presunúť dovnútra seba samého
hex-filled = vyplnené: { $size }
hex-xored = { $size } po XOR — toto nie je šifrovanie
hex-written = { $size } zapísané do { $name }
hex-read-done = { $size } načítané z { $name }
hex-inserted = vložené nuly: { $size }
hex-deleted = zmazané: { $size }
hex-device = { $name }: zariadenie s veľkosťou { $size }, otvorené len na čítanie
asm-title = Asemblovať na { $at } ({ $arch })
asm-no-arch = { $arch } nie je architektúra, pre ktorú vieme asemblovať
asm-written = { $text } — { $bytes }
asm-written-longer = { $text } — { $bytes } (o { $grew } bajtov dlhšie)
screens-title = Okná
screens-panels = Panely
quit-title = Ukončiť FinalCommander
quit-body = Chcete skončiť?
screens-quit-title = Neuložené zmeny
screens-quit-body = Tieto okná obsahujú zmeny, ktoré nikto nezapísal:
macro-recording = Zaznamenáva sa makro · Ctrl+. ho zastaví
macro-empty = nič sa nezaznamenalo
macro-dropped = záznam bol zahodený
macro-key-title = Ktorý kláves
macro-key-body = Zaznamenaných krokov: { $steps }. Stlačte kláves, na ktorý bude makro reagovať, alebo Esc na zahodenie.
macro-describe = Na čo je makro na klávese { $key }
macro-saved = makro na klávese { $key } uložené, krokov: { $steps }
macro-none = zatiaľ žiadne makrá · Ctrl+. jedno zaznamená
macro-list-title = Makrá
macro-play-title = Ako sa má prehrať
macro-play-once = Raz, od miesta, kde je kurzor
macro-play-each = Raz pre každý vybratý súbor
macro-sure-title = Prehrať toto makro
macro-sure-body = Makro na klávese { $key } kopíruje, presúva, maže alebo prepisuje. Prehrať ho?
macro-too-deep = makro nemôže prehrávať samo seba donekonečna
plugin-list-title = Príkazy modulov
plugin-none-title = Nie sú nainštalované žiadne moduly
plugin-none-open = Otvoriť priečinok modulov ({ $dir })
plugin-none-help = Ako moduly fungujú a kde ich nájsť
plugin-help-heading = Moduly
plugin-why-off = vypnutý v Nastavenia → Nastavenia modulov
plugin-why-panel = len v paneloch
plugin-why-viewer = len v zobrazovači
plugin-why-editor = len v editore
plugin-why-local = potrebuje priečinok na diskoch tohto počítača
plugin-why-files = potrebuje súbor pod kurzorom alebo vybratý
plugin-why-masks = len pre { $masks }
plugin-why-bad-mask = jeho maska je chybná: { $error }
plugin-why-here = potrebuje { $names } v tomto priečinku
plugin-why-inside = potrebuje { $names } tu alebo v priečinku vyššie
plugin-cannot = { $label }: { $why }
plugin-running = { $label }…
plugin-on-selected = na vybratých súboroch: { $count }
plugin-failed = { $id }: { $why }
plugin-unknown-command = modul žiadal `{ $id }`, čo nie je príkaz
hex-copied = { $size } skopírované z { $from }
hex-moved = { $size } presunuté z { $from }
menu-calculator = Kalkulačka
menu-terminal = Terminál
calc-title = Kalkulačka
calc-decimal = desiatkovo
calc-hex = hex
calc-octal = osmičkovo
calc-binary = dvojkovo
calc-characters = znaky

# The archive commands (Shift+F3) and the two parts of the order that are not the key.
arc-commands-title = Príkazy pakovača
arc-password = Heslo pre { $name }
arc-op-test = Otestovať { $name }
arc-op-extract-here = Rozbaliť { $name } sem
arc-op-extract-to = Rozbaliť { $name } do…
arc-op-delete = Zmazať { $name }
arc-op-add = Pridať súbory do archívu…
job-test = Testujem archív
msg-dirs-first-on = najprv priečinky
msg-dirs-first-off = priečinky medzi súbormi
msg-selected-first-on = najprv vybraté súbory
msg-selected-first-off = vybraté súbory v poradí

# The change-drive dialog (Alt+F1, Alt+F2): the type column, and its footer.
drive-fixed = Pevný
drive-removable = Vymeniteľný
drive-optical = Optický
drive-network = Sieťový
drive-memory = Pamäť
drive-home = Domov
drive-connection = Pripojenie
drive-objects = Objektové úložisko
drive-netbox = NetBox: uložené relácie
drive-processes = Zoznam procesov
drive-temp = Dočasný panel
temp-added = pridané do dočasného panela: { $count }
proc-none = zoznam bežiacich procesov sa tu nedá prečítať
drive-programs = Nainštalované programy
drive-services = Služby
svc-none = tento počítač nemá správcu služieb, ktorého sa vieme opýtať
temp-not-a-file = riadky tu nie sú súbory: niet čo zmazať
svc-card = Služba
svc-card-id = Názov
svc-card-name = Zobrazený názov
svc-card-state = Stav
svc-card-description = Popis
svc-card-pid = Proces
svc-state-running = beží
svc-state-stopped = zastavená
svc-state-failed = zlyhala
svc-state-other = iný
key-service-start = Štart
key-service-restart = Reštart
key-service-stop = Stop
svc-ask-stop = Zastaviť { $name }?
svc-ask-restart = Reštartovať { $name }?
btn-service-stop = Zastaviť
btn-service-restart = Reštartovať
svc-working = { $name }: pýtam sa systému…
svc-started = { $name } spustená
svc-stopped = { $name } zastavená
svc-restarted = { $name } reštartovaná
svc-needs-root = { $name }: toto vyžaduje práva root — program si ich za vás nevyžiada
svc-failed = { $name }: { $error }
prog-none = nič tu nehovorí, čo je nainštalované
prog-card = Nainštalovaný program
prog-no-uninstall = nič nehovorí, ako odstrániť { $name }
prog-uninstall-ready = príkaz, ktorý odstráni { $name }, je na príkazovom riadku, nespustený
prog-card-name = Názov
prog-card-version = Verzia
prog-card-publisher = Vydavateľ
prog-card-size = Veľkosť na disku
prog-card-installed = Nainštalované
prog-card-location = Priečinok
prog-card-uninstall = Odstraňuje sa cez
drive-sockets = Zoznam socketov
sock-none = nič tu nepočúva alebo sa tabuľka nedá prečítať
sock-no-process = nič nepovie, ktorý proces drží tento socket
proc-card = Proces
proc-gone = { $name } už nebeží
proc-card-name = Názov
proc-card-pid = ID procesu
proc-card-parent = Spustil ho
proc-card-user = Užívateľ
proc-card-state = Stav
proc-card-threads = Vlákna
proc-card-memory = Pamäť
proc-card-mapped = Mapované
proc-card-bits = Bitová šírka
proc-card-started = Spustený
proc-card-exe = Spustiteľný súbor
proc-card-cwd = Priečinok
proc-card-command = Príkazový riadok
proc-card-group = Skupina
proc-card-sid = SID
proc-card-priority = Priorita
proc-card-session = Relácia
proc-card-uptime = Beží už
proc-card-cpu-user = Čas procesora, vlastný
proc-card-cpu-kernel = Čas procesora, v jadre
proc-card-handles = Otvorené handle
proc-card-faults = Výpadky stránok
proc-card-private = Súkromná pamäť
proc-card-swap = Odložené na disk
proc-card-io-read = Prečítané
proc-card-io-write = Zapísané
proc-card-memory-peak = Pracovná množina, maximum
proc-card-private-resident = Pracovná množina, súkromná
proc-card-swap-peak = Stránkovací súbor, maximum
proc-card-virtual = Virtuálne bajty
proc-card-virtual-peak = Virtuálne bajty, maximum
proc-card-paged-pool = Stránkovaný pool
proc-card-nonpaged-pool = Nestránkovaný pool
proc-card-io-other = Iné V/V
proc-card-io-read-ops = Operácie čítania
proc-card-io-write-ops = Operácie zápisu
proc-card-io-other-ops = Iné operácie
proc-kill-title = Ukončiť proces
proc-kill-ask = Ukončiť { $name } [{ $pid }]? Čo neuložil, sa stratí.
proc-killing = Ukončujem { $name }…
proc-killed = { $name } ukončený
proc-kill-failed = { $name } sa nepodarilo ukončiť: { $why }
proc-card-env = Prostredie
proc-card-modules = Načítané moduly
proc-card-version = Verzia
proc-card-description = Popis
drives-hint = ↑↓ výber · 0-9 panely · Ctrl+1..9 stĺpce · F9 všetky · Ctrl+R obnoviť
drives-title = Zmeniť jednotku: čo zobraziť
drives-col-type = typ
drives-col-device = zariadenie
drives-col-label = názov disku
drives-col-fs = súborový systém
drives-col-size = veľkosti
drives-col-removable = vymeniteľné jednotky
drives-col-panels = panely, ktoré nie sú jednotky
drives-col-optical = optické jednotky
drives-col-network = sieťové zdieľania
drives-shown = zobrazuje sa: { $what }
drives-hidden = skrýva sa: { $what }
drives-reread = jednotky boli načítané znova
bookmarks-hint = Enter prejsť · Ins uložiť · Del odstrániť · F4 názov · Esc zatvoriť

# The search dialog (F7), which asks the whole question in one place.
search-title = Hľadať
search-for = Hľadať
search-mode = Ako
search-mode-text = text
search-mode-hex = hex
search-mode-regex = regulárny výraz
search-case = Rozlišovať malé a VEĽKÉ písmená
search-words = Len celé slová
btn-find-next = Nájsť ďalší
btn-find-prev = Nájsť predchádzajúci
# The help: the chapters' own names, and the note on a chapter nobody has translated yet
help-chapter-about = Čo to je
help-chapter-panels = Panely
help-chapter-files = Súbory: kopírovanie, presun, hľadanie, archívy
help-chapter-viewer = Zobrazovač
help-chapter-editor = Editor, hex panel a asembler
help-chapter-network = Servery
help-chapter-config = Konfigurácia, klávesy a farby
help-chapter-keys = Klávesy, ako sú priradené teraz
help-chapter-commands = Každý príkaz a jeho klávesy
help-chapter-options = Každé nastavenie a jeho hodnota
help-here-panels = Panely
help-here-viewer = Zobrazovač
help-here-editor = Editor
help-here-hex = Hex editor
help-here-dialog = Tento dialóg
help-in-english = Táto kapitola ešte nebola preložená do vášho jazyka, preto nasleduje po anglicky. Klávesy a nastavenia, ktoré spomína, sú vo všetkých jazykoch rovnaké.
help-about-byline = FinalCommander { $version } — autor Matteo Baccan
key-ok = OK
key-cancel = Storno
key-dialog-move = Posun
key-tree = Strom
key-config = Konfig
browse-local-only = súbor sa vyberá z tohto počítača; cesta na serveri sa píše a Tab ju dopĺňa
key-browse = Vybrať
table-prompt-filter = Riadky obsahujúce
table-sorted = zoradené podľa { $column }
table-sorted-back = zoradené podľa { $column }, opačne
table-unsorted = späť v poradí, v akom ich má súbor
table-filtered = zostalo riadkov: { $rows }
table-unfiltered = znova všetky riadky
table-cut-sorted = z prvých { $rows } riadkov, čo je všetko, čo sa načítalo
table-too-many = priveľa záznamov na triedenie alebo filtrovanie
table-copied =
    { $rows ->
        [one] jeden riadok v schránke
        [few] { $rows } riadky v schránke
       *[other] { $rows } riadkov v schránke
    }
table-headed = prvý riadok je hlavička
table-not-headed = prvý riadok sú dáta: stĺpce sa volajú podľa čísla
table-named-fields = tento formát pomenúva polia sám: niet hlavičky, ktorú by bolo treba hádať

# Splitting and combining (SPEC 03 §9)
split-title = Rozdeliť súbor
split-target = Kam
split-size = Každá časť
split-size-hint = 1440K, 700M, 4G — posledná časť je to, čo zostane
split-count = Alebo na
split-count-hint = častí — číslo tu má prednosť pred veľkosťou
split-bad-count = { $count } nie je počet častí: dve alebo viac
split-local-file = Delenie a spájanie pracujú so súborom na tomto počítači
split-bad-size = „{ $size }“ nie je veľkosť: napíšte ju ako 700M alebo 4G
combine-not-a-piece = To nie je časť: časti rozdeleného súboru sa končia na .001, .002, … alebo majú pri sebe .crc
pieces-working = Delím alebo spájam: { $done } z { $total }
pieces-cancelled = Zastavené, zapísané sa vzalo späť
split-done = zapísaných častí: { $parts }, a vedľa nich { $list }
combine-done-checked = { $name } spojený z { $parts } častí, { $bytes }, a súhlasí so svojím .crc
combine-done-unchecked = { $name } spojený z { $parts } častí, { $bytes } — nebolo .crc, podľa ktorého by sa overil
pieces-failed = Nevykonané: { $error }

# The branch view (SPEC 02 §3d)
branch-reading = Čítam každý súbor pod { $path }…
branch-title = Vetva: { $path }
branch-done = súborov: { $count }, zo všetkých priečinkov pod týmto · Ctrl+Alt+B sa vráti
branch-truncated = súborov: { $count }, tam sa prechádzanie zastavilo: vetva je väčšia, než môže byť užitočný zoznam
branch-not-here = Zobrazenie vetvy patrí priečinku, a tento panel je zoznam

## The release check (SPEC 11 §5).
menu-check-update = Skontrolovať a&ktualizácie
opt-update-check = Raz denne skontrolovať nové vydanie
btn-open-page = Otvoriť stránku
update-checking = Pýtam sa GitHubu na najnovšie vydanie…
update-title = Aktualizácie
update-available = Vyšiel FinalCommander { $version } (toto je { $current }).
update-available-status = Vyšiel FinalCommander { $version }: Nastavenia → Skontrolovať aktualizácie
update-current = Toto je najnovšie vydanie, { $version }.
update-failed = Nepodarilo sa zistiť najnovšie vydanie: { $why }

## Duplicate files (SPEC 06 §13).
menu-dupes = Nájsť duplicitné súbory
dupes-title = Duplicitné súbory
dupes-recursive = Hľadať v podpriečinkoch
dupes-min-size = Najmenší súbor
dupes-min-size-hint = Veľkosť, ako ju píšu filtre: 1, 100K, 5M. Prázdne súbory sa nikdy neporovnávajú.
dupes-select = Označiť každú kópiu okrem najstaršej
btn-search = Hľadať
dupes-not-here = Hľadanie duplikátov sa začína od priečinka alebo od toho, čo je vybraté.
dupes-bad-size = Nie je veľkosť: { $size }
dupes-looking = Hľadám duplicitné súbory…
dupes-progress = Duplikáty: prechod { $pass } z 3, { $done }/{ $total }
dupes-none = Žiadne duplikáty medzi { $count } súbormi.
dupes-panel = Duplikáty: skupín { $groups }
dupes-done = skupín duplikátov: { $groups }, v kópiách { $size }.
dupes-truncated = Zastavené predčasne: zatiaľ skupín { $groups }, v kópiách { $size }.

## SQLite queries (SPEC 04 §4b).
sql-empty = Napíšte dopyt za sql:, napríklad sql: SELECT * FROM t
sql-no-database = sql: potrebuje databázu SQLite: vojdite do nej alebo na ňu dajte kurzor.
sql-running = Vykonávam dopyt…
sql-failed = Dopyt zlyhal: { $why }

## Asking a language model (SPEC 06 §14).
ai-none = Nie je nastavená žiadna služba AI: pridajte blok [[ai]] do fcmd.toml (name, url, model, api, token_env).
ai-title = Opýtať sa AI
ai-no-thread = niet odpovede, na ktorú by sa dalo nadviazať: najprv sa opýtajte cez `ai:`, alebo dajte kurzor na odpoveď v priečinku odpovedí
ai-nothing-to-explain = niet tu čo vysvetliť: otvorte súbor v zobrazovači alebo editore, alebo sa opýtajte hneď po príkaze, ktorý zlyhal
ai-explain-question = Čo je toto, z { $name }? Vysvetli to stručne.
ai-tokens = tokeny: { $in } dnu, { $out } von
ai-none-panel = Zatiaľ žiadna služba AI — F7 jednu pridá
ai-service-removed = { $name } odstránená
ai-probing = Prihlasujem sa k { $name } na { $host }…
ai-probe-ok = { $host } odpovedal HTTP { $status } za { $ms } ms, modelov: { $models } — napíšte otázku a stlačte Enter
ai-probe-no-model = (model { $model } medzi nimi nie je)
ai-row-context = kontext
ai-row-conversation = konverzácia
ai-session-title = AI: { $name } · { $model } — súborov v kontexte: { $files }
ai-context-added = { $count ->
        [one] Jeden súbor
        [few] { $count } súbory
       *[other] { $count } súborov
    } v kontexte
ai-context-nothing = Niet čo pridať: do kontextu idú len súbory z tohto disku
ai-command-ready = príkaz je na riadku — prečítajte si ho, kým stlačíte Enter
ai-commit-question = Napíš správu commitu pre pripravené zmeny.
ai-staged-diff = pripravený diff
ai-send-diff = Pripravený diff ide na { $host }
ai-send-files = Súbory ({ $count }, { $size }) sa odošlú na { $host } spolu s otázkou.
ai-asking = Pýtam sa { $name }…
ai-busy = { $name } premýšľa… { $secs } s · Esc zastaví
ai-conversation-new = Nová konverzácia: začne ju ďalšia otázka.
ai-live-empty = Napíšte otázku do príkazového riadka: pôjde na { $name } a konverzácia sa tu bude priebežne zobrazovať.
ai-failed = AI neodpovedala: { $why }
ai-stopped = zastavené: otázka pre { $name } bola zahodená a nič sa nezapísalo
ai-context-title = Súbory v kontexte
ai-context-all = Všetky
ai-context-dropped = { $count ->
        [one] jeden súbor vyradený z kontextu
        [few] { $count } súbory vyradené z kontextu
       *[other] { $count } súborov vyradených z kontextu
    }
ai-context-hint = Enter alebo Del: vyradiť z kontextu — súbor zostane na disku · F3: čo sa odosiela · Esc
ai-threads-title = Konverzácie
ai-threads-hint = Enter: pokračovať v tejto konverzácii · Esc
ai-thread-resumed = pokračujem v { $name }
ai-next-estimate = ďalšia ≈ { $tokens }
ai-next-over = ďalšia ≈ { $tokens }, viac, než model pojme
ai-send-estimate = Spolu asi { $tokens } tokenov, s doterajšou konverzáciou.
ai-send-estimate-of = Spolu asi { $tokens } tokenov, s doterajšou konverzáciou, z { $max }, ktoré model pojme.
ai-send-over = Spolu asi { $tokens } tokenov: viac než { $max }, ktoré model pojme, takže začiatok sa môže odrezať. F8 vyradí súbor.
ai-send-question = Otázka ide na { $host }.
btn-send = Odoslať

## Git (SPEC 06 §15).
menu-git = Git
git-title = Git
git-status = Čo sa zmenilo
git-log = Log
git-blame = Blame { $name }
git-branches = Vetvy
git-local-only = git pracuje s priečinkom na tomto disku.
git-running = Pýtam sa gitu…
git-failed = git: { $why }
git-clean = Nič sa nezmenilo.
git-status-title = Zmenené: pripravené { $staged }, nepripravené { $unstaged }
git-side-conflict = konflikt
git-side-staged = pripravené
git-side-changed = nepripravené
git-side-untracked = nesledované
git-kind-modified = zmenený
git-kind-added = nový
git-kind-deleted = zmazaný
git-kind-renamed = premenovaný
git-kind-copied = skopírovaný
git-kind-type = zmenený typ
git-kind-conflict = na vyriešenie
git-as-tree = Zobraziť zmeny ako strom
git-as-list = Zobraziť zmeny ako zoznam
git-version-head = posledný commit
git-version-index = pripravené
git-version-disk = na disku
git-discard-title = Zahodiť zmeny
git-discard-body = Zahodiť zmeny { $count ->
        [one] jedného súboru
       *[other] { $count } súborov
    }?
git-discard-worktree = späť na index:
git-discard-both = späť na posledný commit:
git-discard-added = von z indexu, zostane na disku:
git-discard-trash = do Koša:
git-discard-warning = Čo sa vráti na index alebo posledný commit, sa nedá vziať späť.
git-hint = Enter: vybrať · Esc: zatvoriť

## The editor's character list and word completion.
charmap-title = Znaky
charmap-hint = Píšte na zúženie (znak alebo jeho kód, U+00E9) · Enter: vložiť · Esc: späť
editor-no-completion = Žiadne iné slovo v súbore sa takto nezačína.

## A stored session's address, copied (NetBox's Generate URL).
net-url-copied = Skopírované { $url }

## The system log (SPEC 06 §16).
menu-syslog = Systémový log
syslog-reading = Čítam systémový log…

## Deleting lines by pattern (FAR #10).
editor-prompt-delete-matching = Odstrániť riadky vyhovujúce (! na začiatku: ponechať len tie)
editor-lines-deleted = Odstránených riadkov: { $count }

## The copy dialog's size policy (FAR #493).
copy-conflict-size = prepísať, ak sa líši veľkosť

# Copy dialog: what an error does (FAR's issue #597)
copy-on-error = Ak sa niektorý nedá skopírovať
copy-error-ask = Opýtať sa
copy-error-skip = Preskočiť a pokračovať
copy-error-stop = Zastaviť kopírovanie

# Attributes dialog: the times copied from one file to another (FAR's issue #241)
attr-times-copied = Časy skopírované: Ctrl+Shift+V ich vloží do dialógu iného súboru
attr-times-none = Zatiaľ nič skopírované: najprv Ctrl+Shift+C v dialógu súboru
attr-times-pasted = Časy vložené: OK ich nastaví

# Editor: replacements kept by name and recent ones (FAR's issue #282)
replace-tpl-title = Nahradenia
replace-tpl-hint = Enter spustí · Ins uloží nedávne pod názvom · Del zabudne uložené · Esc
replace-tpl-name = Uložiť toto nahradenie ako
replace-tpl-kept = Uložené ako {$name}
replace-tpl-empty = Zatiaľ žiadne nahradenie: najprv Ctrl+F7

# Panel settings: a view and a sort per folder (FAR's issue #389)
opt-remember-view = Každý priečinok si drží vlastné zobrazenie a triedenie

# Session editor: a cap on a copy's speed to or from the site
site-max-kbps = Limit rýchlosti, KB/s (prázdne: žiadny)

# Session editor: IMAP, read only
site-protocol-imap = IMAP (pošta, len na čítanie)
site-protocol-imaps = IMAPS (pošta cez TLS, len na čítanie)

# The AI services as a list, like the sessions
ai-services-title = Služby AI
ai-services-hint = Enter: tej sa pýta ai: · Ins nová · F4 zmeniť · Del odstrániť · Esc
ai-service-title = Služba AI
ai-field-name = Názov
ai-field-api = Protokol
ai-field-url = Adresa
ai-field-model = Model
ai-field-token-env = Kľúč v premennej
ai-field-token-note = Prázdne: kľúč sa vyžiada raz za reláciu a nikam sa nezapíše.
ai-field-system = Čo sa jej povie ako prvé
ai-service-incomplete = Služba potrebuje názov a adresu
ai-service-default = ai: sa teraz pýta {$name}
ai-key-prompt = Kľúč pre {$name} (drží sa do ukončenia)
ai-api-openai = OpenAI a kompatibilné
ai-api-anthropic = Anthropic
ai-api-ollama = Ollama (na tomto počítači)
menu-ai = Služby AI

# The secrets file
vault-password = Heslo k súboru tajomstiev fcmd
vault-new = Uložiť tento kľúč do šifrovaného súboru tajomstiev fcmd: zvoľte jeho heslo (prázdne: len pre túto reláciu)
vault-refused = Súbor tajomstiev sa neotvoril: {$why}

# The programs from elsewhere
drive-tools = Externé programy
drive-ai = AI
drive-git = Git
tools-found = nájdený
tools-missing = nenainštalovaný
tools-summary = nájdených externých programov: { $found } z { $total }; F3 povie, na čo ktorý je, F4 nainštaluje chýbajúci
tools-card = Externý program
tools-card-program = Program
tools-card-where = Kde
tools-card-version = Verzia
tools-card-for = Používa sa na
tools-card-install = Inštalácia
tools-card-why = Prečo ho fcmd spúšťa, namiesto toho, aby to robil sám: F1, „Programy odinakiaľ“.
tools-install-any = nainštalujte balík, ktorý poskytuje { $name }, a uistite sa, že je v PATH
tools-already = { $name } tu už je
tools-for-archives = archívy: { $what }
tools-for-decompile = Shift+F6 v zobrazovači na { $what }
tools-for-git = panel Git (Ctrl+Alt+G) a vetva vo výzve
tools-for-video = snímky videa v zobrazovači a v rýchlom info (F3, Ctrl+Q)
tools-for-open = otvorenie odkazu v prehliadači
tools-for-syslog = Príkazy → Systémový log
tools-for-services = panel služieb
tools-card-site = Webová stránka
tools-no-package = Nie je známy správca balíkov na tomto počítači, ktorý by mal { $name }. Otvoriť stránku na stiahnutie?
tools-install-ask = Nainštalovať { $name } spustením v termináli:
tools-install-how = V termináli si sudo vypýta vaše heslo. Ctrl+Alt+E sa vráti do panelov a Ctrl+R v tomto paneli sa pozrie znova.
tools-install-else = Ak to zlyhá: { $url }
tools-install-failed = fcmd: { $name } sa nenainštaloval. Stiahnite ho z { $url }
tools-installing = inštalujem { $name } v termináli: Ctrl+Alt+E sa vráti do panelov, Ctrl+R tam sa pozrie znova
btn-install = Inštalovať

# Plugins configuration
plugins-config-title = Moduly
plugins-config-hint = Enter: zapnúť alebo vypnúť · F3 čo to je a čo prináša · Esc
plugins-broken = nenačítal sa — F3 povie prečo
plugins-cannot-toggle = modul, ktorý sa nenačítal, sa nedá zapnúť: F3 povie, čo je s ním zlé
plugins-on = { $id } je zapnutý: jeho príkazy sú teraz v Ctrl+Shift+F11
plugins-on-restart = { $id } je zapnutý: jeho príkazy sú teraz v Ctrl+Shift+F11, ostatné prinesie od ďalšieho spustenia
plugins-off = { $id } je vypnutý: jeho príkazy z Ctrl+Shift+F11 zmizli
plugins-off-restart = { $id } je vypnutý: jeho príkazy z Ctrl+Shift+F11 zmizli, ostatné zmizne pri ďalšom spustení
plugins-state-on = zapnutý
plugins-state-off = vypnutý
plugins-card-id = Id
plugins-card-name = Názov
plugins-card-version = Verzia
plugins-card-author = Autor
plugins-card-about = O module
plugins-card-dir = Priečinok
plugins-card-state = Stav
plugins-card-brings = Prináša
plugins-card-commands = Príkazy (Ctrl+Shift+F11):
plugins-card-notes = Hlásenia pri načítaní:
plugins-none = nenašli sa žiadne moduly · bývajú v { $dir }, každý vo vlastnom priečinku s plugin.toml

# The git panel
git-file-log = Log súboru { $name }
git-commit = Commitnúť pripravené…
git-pull = Pull
git-push = Push
git-pushing = Prebieha push…
git-pulling = Prebieha pull…
git-commit-prompt = Správa commitu
git-commit-empty = Commit potrebuje správu.
git-already-on = Na vetve { $branch } už ste.
git-switch-ask = Prepnúť na { $branch }? Súbory, v ktorých sa vetvy líšia, sa na disku zmenia.
git-done = git: { $said }
git-branches-title = Vetvy: { $count }
git-log-title = Log: { $count }
git-log-of-title = Log pre { $of }: { $count }
btn-switch = Prepnúť
key-git = Git
key-git-diff = Diff
key-git-stage = Priprav
key-git-unstage = Vyraď
key-git-discard = Zahoď
key-git-commit = Commit
key-git-show = Ukáž
key-git-log = Log
key-git-changes = Zmeny
key-git-pull = Pull
key-git-push = Push
key-git-branches = Vetvy

undo-title = Vrátiť späť
btn-undo = Vrátiť
menu-undo = Vrátiť poslednú operáciu
menu-strip-meta = Odstrániť metadáta obrázkov…
undo-nothing = niet čo vrátiť: od spustenia programu žiadne kopírovanie, presun, zmazanie do Koša ani nový priečinok
undo-copy =
    { $count ->
        [one] Poslať súbor, ktorý kopírovanie vytvorilo, do Koša:
        [few] Poslať { $count } súbory, ktoré kopírovanie vytvorilo, do Koša:
       *[other] Poslať { $count } súborov, ktoré kopírovanie vytvorilo, do Koša:
    }
undo-move =
    { $count ->
        [one] Presunúť súbor späť, kde bol:
        [few] Presunúť { $count } súbory späť, kde boli:
       *[other] Presunúť { $count } súborov späť, kde boli:
    }
undo-trash =
    { $count ->
        [one] Obnoviť súbor z Koša:
        [few] Obnoviť { $count } súbory z Koša:
       *[other] Obnoviť { $count } súborov z Koša:
    }
undo-mkdir = Odstrániť nový priečinok, ak je ešte prázdny:
strip-title = { $count ->
        [one] Metadáta jedného obrázka
       *[other] Metadáta { $count } obrázkov
    }
strip-all = Všetko — text, EXIF, XMP, IPTC, komentáre, C2PA
strip-ai = Len to, čo zapísal obrazový model — prompt, workflow, nastavenia
strip-camera = Všetko okrem EXIF fotoaparátu a copyrightu — bez GPS
strip-hint = Enter: odstrániť; obrázok sa neprekóduje, Ctrl+Alt+Z to vráti · Esc
strip-nothing = na tomto disku nie je vybratý ani pod kurzorom žiadny PNG, WebP ani JPEG
strip-done = { $count ->
        [one] jeden obrázok
        [few] { $count } obrázky
       *[other] { $count } obrázkov
    } bez metadát (odstránených blokov: { $blocks }), bez čohokoľvek na odstránenie: { $clean } · Ctrl+Alt+Z ich vráti
undo-restore = { $count ->
        [one] Vrátiť obrázok aj s metadátami:
        [few] Vrátiť { $count } obrázky aj s metadátami:
       *[other] Vrátiť { $count } obrázkov aj s metadátami:
    }
undo-done =
    { $count ->
        [one] jeden názov späť, kde bol
        [few] { $count } názvy späť, kde boli
       *[other] { $count } názvov späť, kde boli
    }
slow-drives = zoznam jednotiek trval { $secs } s
slow-drives-late = zoznam jednotiek čakal { $secs } s: { $drives } neodpovedali včas a zobrazujú sa bez podrobností — sieťová jednotka, ktorej server nie je dostupný?
slow-listing = čítanie { $what } trvalo { $secs } s
# editor search and undo (0.41)
btn-find-all = Všetky výskyty
btn-replace-all = Nahradiť všetky
btn-replace-one = Nahradiť ďalší
editor-replace-title = Nahradiť
editor-replace-style = Zachovať štýl (foo → bar, Foo → Bar, FOO → BAR)
editor-replace-groups = V regulárnom výraze sú $1 a { "${name}" } to, čo zachytila skupina, $$ je dolár
editor-matches-title = Riadky s `{ $query }`: { $count }
editor-matches-hint = Píšte na zúženie · Enter: ísť na riadok · Esc: späť


# sorting and extended attributes (0.41)
opt-sort-case = Veľké a malé písmená pri triedení
opt-sort-case-auto = ako tento systém
opt-sort-case-yes = rozlišovať
opt-sort-case-no = jedna abeceda
copy-keep-xattrs = Zachovať rozšírené atribúty

# editor conversions, disk diff and guides (0.41)
editor-convert-title = Zapísať súbor ako
editor-convert-hint = Píšte na zúženie · Enter: konvertovať (F2 to zapíše, Ctrl+Z vráti) · Esc: späť
editor-convert-eol = Koniec riadka: { $eol }
editor-convert-bom-on = Značka poradia bajtov (BOM): zapisuje sa
editor-convert-bom-off = Značka poradia bajtov (BOM): nezapisuje sa
editor-convert-page = Tabuľka znakov: { $page }
editor-converted-eol = { $name }: pri ďalšom uložení sa riadky skončia na { $what }
editor-converted-bom-on = { $name }: ďalšie uloženie zapíše značku poradia bajtov
editor-converted-bom-off = { $name }: ďalšie uloženie nezapíše značku poradia bajtov
editor-converted-page = { $name }: ďalšie uloženie zapíše { $what }
editor-convert-same = Súbor je už tak zapísaný
editor-convert-no-bom = { $page } nemá značku poradia bajtov: majú ju len UTF-8 a UTF-16
editor-convert-lossy = { $page } nepojme každý znak z { $name }: nič sa nezmenilo
btn-reload = Znova načítať
btn-what-changed = Čo sa zmenilo
editor-diff-buffer = v editore
editor-diff-hint = Toto porovnanie len ukazuje, čo sa zmenilo — Esc sa vráti k otázke


# job journal and saved syncs (0.41)
journal-left = Kopírovania nedokončené z poslednej relácie: { $count } · Alt+Shift+F6 ich vypíše
journal-title = Nedokončené úlohy
journal-hint = Enter pokračovať · Del zahodiť · Esc zatvoriť
journal-none = Žiadna úloha nezostala nedokončená
journal-row = { $kind }: { $what } → { $target } · hotových súborov: { $done }
journal-row-unreadable = nedá sa prečítať: { $file } ({ $error })
journal-unreadable = { $file }: nie je to žurnál úlohy, ktorý táto verzia vie čítať ({ $error }). Alt+Shift+F6 ho vypíše a Del ho tam odstráni
journal-resumed = Pokračujem v { $kind }: hotových súborov už bolo { $done }
journal-discarded = Nedokončená úloha bola zahodená aj s tým, čo napoly zapísala
journal-cannot-resume = Tento žurnál sa nedá prečítať; Del ho odstráni
menu-jobs-left = Nedokončené úlohy
sync-report = Zapísať správu o behu do (súbor)
sync-name = Uložiť túto dvojicu ako (názov)
sync-saved-button = Uložené…
sync-saved-title = Uložené synchronizácie
sync-saved-hint = Enter spustiť · F4 zmeniť · Del zabudnúť · Esc zatvoriť
sync-saved-none = Zatiaľ nie je uložená žiadna synchronizácia: pomenujte ju v dialógu (Ctrl+Shift+Y)
sync-saved-removed = { $name } už nie je uložená
sync-report-local = Správa musí byť súbor na tomto počítači
sync-report-written = Správa o synchronizácii je v { $path }
sync-report-not-written = Správu sa nepodarilo zapísať do { $path }: { $error }
sync-report-title = Správa o synchronizácii
sync-report-name = Názov: { $name }
sync-report-left = Vľavo: { $path }
sync-report-right = Vpravo: { $path }
sync-report-to-right = Skopírované zľava → doprava: { $count }
sync-report-to-left = Skopírované sprava → doľava: { $count }
sync-report-cut-right = Odstránené vpravo: { $count }
sync-report-cut-left = Odstránené vľavo: { $count }
sync-report-skipped = Ponechané, hoci sa líšia: { $count }
sync-report-failed = Zlyhalo: { $count }
sync-report-stopped = Zastavené na polceste, takže tieto mohli aj nemuseli prebehnúť: { $count }
sync-report-not-done = Nevykonané, lebo skorší krok zlyhal alebo bol zastavený: { $count }
menu-sync-saved = Uložené synchronizácie

# hex editor search
hex-search-wrapped = `{ $query }`: nájdené po začatí od druhého konca súboru
hex-replace-title = Nahradiť
hex-replace-with = Nahradiť čím
btn-replace = Nahradiť
hex-replace-ask = Nahradiť { $was } na { $at } za { $with }?
hex-replace-length = náhrada má { $with } bajtov a nájdené { $was }: posunulo by to všetko za tým — najprv stlačte Ins pre režim vkladania

# pixels and video (0.41)
opt-viewer-pixels = Obrázky
opt-viewer-pixels-auto = skutočné pixely tam, kde ich terminál kreslí
opt-viewer-pixels-kitty = grafický protokol kitty
opt-viewer-pixels-iterm = vložené obrázky iTerm2
opt-viewer-pixels-sixel = sixel
opt-viewer-pixels-off = vždy polovičné bloky
video-needs-program = toto je video ({ $what }): jeho snímky sa kreslia cez `{ $program }`, ktorý na tomto počítači nie je nainštalovaný — Shift+F11 aj tak prečíta kontajner
video-no-frame = žiadna snímka z tohto videa: { $why }
video-playing = prehráva sa pri { $fps } snímkach za sekundu, bez zvuku — Space pozastaví
video-paused = pozastavené na { $at } — Space pokračuje, PgUp a PgDn krokujú po desatine
video-ended = video skončilo — Space ho prehrá znova


# finding things
menu-saved-searches = Uložené hľadania
drive-searches = Uložené hľadania
find-keys = F2 uloží toto hľadanie pod názvom · F9 uložené
find-save-name = Uložiť toto hľadanie ako
find-saved = hľadanie uložené ako { $name }
find-saved-none = zatiaľ nie je uložené žiadne hľadanie: F2 v dialógu hľadania ho uloží
find-saved-title = Uložené hľadania — Enter otvorí, F4 upraví, Del zabudne
find-saved-running = hľadám { $name }…
find-saved-gone = hľadanie { $name } zabudnuté
find-loaded = hľadanie { $name } načítané
find-fuzzy = Približný názov: písmená vyššie, v poradí, najlepšia zhoda prvá
find-meta = Metadáta (artist=bowie; camera=)
find-index = Opýtať sa indexu systému namiesto prechádzania (nové súbory môžu chýbať)
tools-for-index = index, ktorého sa Alt+F7 môže opýtať namiesto prechádzania
menu-frecent = Časté priečinky
frecent-none = zatiaľ nebol navštívený žiadny priečinok
frecent-gone = { $path } vyradený z častých priečinkov
frecent-hint = píšte písmená cesty · Enter prejsť tam · Del zabudnúť · Esc zatvoriť

# ai third round
ai-redact-off = Maskovanie je vypnuté: všetko ide tak, ako je
ai-redact-none = Nenašlo sa žiadne tajomstvo na zamaskovanie
ai-redact-count = Tajomstvá zamaskované pred odoslaním: { $count } ({ $kinds })
ai-redacted = zamaskovaných tajomstiev: { $count }
ai-preview-title = Čo ide na { $name } ({ $host })
ai-preview-instruction = Inštrukcia
ai-preview-earlier = Doterajšia konverzácia
ai-preview-question = Otázka a jej súbory
ai-preview-key = F3 ukáže, čo sa odosiela
ai-preview-reading = Čítam, čo by sa odoslalo…
key-ai-redact = Maskuj
ai-redact-now-on = Maskovanie je zapnuté: kľúče, tokeny, súkromné kľúče a heslá sa pred odoslaním zamaskujú
ai-redact-now-off = Maskovanie je vypnuté: čo sa odosiela, ide tak, ako je
ai-panel-redact-on = tajomstvá maskované
ai-panel-redact-off = tajomstvá nemaskované
key-ai-local = Lokál
ai-row-refused = nie je tento počítač: odmietnuté
ai-panel-local-only = len lokálne
ai-local-now-on = Len lokálne je zapnuté: služba, ktorá nie je na tomto počítači, sa odmietne a nič sa jej nepošle
ai-local-now-off = Len lokálne je vypnuté: možno sa pýtať každej služby v zozname
ai-local-refused = Len lokálne je zapnuté: { $name } odpovedá na { $host }, čo nie je tento počítač — nič sa neodoslalo (F6 v paneli AI to vypne)
ai-rename-prompt = Čím sa má stať { $count } názvov? (slovami)
ai-send-names = Inštrukcia a { $count } z { $total } názvov — len názvy, nič zo súborov — idú na { $host }.
ai-rename-ready = vzor modelu je v dialógu: prečítajte náhľad, opravte, čo je zle, a premenuje až OK
ai-rename-waiting = vzor premenovania od modelu je pripravený: Ctrl+Shift+F6 s ním otvorí dialóg
ai-rename-unreadable = model neodpovedal vzorom premenovania: jeho odpoveď je v priečinku odpovedí (ai:) a nič sa nepremenovalo
ai-rewrite-prompt = Čo sa má stať s označeným blokom? (slovami)
ai-rewrite-no-block = najprv označte blok (Shift+šípky): prepis nahradí presne to, čo je označené, a stĺpcový blok nemá tvar, do ktorého by sa dalo vrátiť
ai-rewrite-read-only = tento súbor je tu len na čítanie: prepis by nemal kam ísť
ai-rewrite-before = blok
ai-rewrite-after = ako ho prepísal model
ai-rewrite-hint = Prepis modelu oproti bloku — Enter ho dá na miesto bloku, Esc nechá blok, ako je
diff-hint-rewrite = Enter nahradiť blok · Esc nechať
ai-rewrite-applied = blok bol nahradený prepisom modelu — jedno Ctrl+Z ho vráti
ai-rewrite-left = blok zostal, ako je
ai-rewrite-stale = blok už nie je taký, aký bol pri žiadosti o prepis: nič sa nenahradilo — verzia modelu je v priečinku odpovedí (ai:)
ai-rewrite-same = model vrátil blok tak, ako je: niet čo nahradiť
ai-answer-kept = odpoveď je { $name } v priečinku odpovedí (ai:) — editor zostal vpredu
ai-failure-question = Tento príkaz skončil s kódom { $code }: `{ $command }` — prečo zlyhal a čo by to opravilo? Stručne.
ai-send-failure = Príkaz, ktorý zlyhal, a posledných { $lines } riadkov jeho výstupu idú na { $host }.

# archives and jobs
arc-test-whole = { $name }: otestovaných položiek { $files }, { $bytes } — všetky v poriadku
arc-test-bad-title = { $name }: { $bad } z { $files } položiek nevyšlo v poriadku

# extending it
plugin-column-failed = { $column }: { $error } — jeho stĺpec zostane prázdny pri súboroch, pre ktoré neodpovedal
plugins-card-columns = Stĺpce (ako ich pomenúva [[column_set]]):
usermenu-far-row = Čo sa z FarMenu.ini prečítalo inak ({ $count })
usermenu-far-notes = menu pochádza z FarMenu.ini programu FAR: vecí prečítaných inak: { $count } — posledný riadok ich vypíše
usermenu-far-not-run = Nič sa nespustilo.
find-age = Zmenené, pred koľkými dňami (7, >=30, 7-30)
find-bad-age = `{ $text }` nie je vek v dňoch — { $error }
find-label-age = { $days } d
find-keys-ai = Ctrl+Alt+Q povie hľadanie slovami: jazykový model vyplní tieto riadky
ai-find-prompt = Čo hľadáte? (slovami)
ai-send-words = Tieto slová — a nič z panelov: žiadny názov, cesta ani súbor — idú na { $host }.
ai-find-ready = hľadanie od modelu je v dialógu: prečítajte riadky, opravte, čo je zle, a hľadá až OK
ai-find-waiting = hľadanie od modelu je pripravené: Alt+F7 s ním otvorí dialóg
ai-find-unreadable = model neodpovedal hľadaním: jeho odpoveď je v priečinku odpovedí (ai:) a nič sa nehľadalo
ai-find-left-out = { $said } — NIE je v riadkoch, lebo to nevedia vyjadriť: { $what }
# treemap and compare
usage-map-title = Využitie disku, celý strom: { $path }
usage-map-walking = Meriam… súborov: { $files }, zatiaľ { $size }
usage-map-where = { $path } · { $size } · súborov: { $files }
usage-map-unreadable = neprečítaných priečinkov: { $count }
usage-map-rest = menších súborov: { $count }
usage-map-others = ostatných: { $count }
usage-map-empty = Nič v tomto priečinku nezaberá miesto
usage-map-chosen = { $name } · { $size } · { $share }%
usage-map-hint = ←↑↓→ · Enter dnu · BkSp hore · Tab zoznam · G panel tam · Esc
usage-map-hint-list = ↑↓ · Enter dnu · BkSp hore · Tab mapa · G panel tam · Esc
usage-map-hint-walking = Esc zastaví
usage-map-top = Toto je priečinok, ktorý sa meral: nad ním tu nič nie je
usage-map-no-file = Tento riadok je veľa malých súborov, nie jeden, na ktorý môže panel prejsť
bin-comparing = Porovnávam oba súbory bajt po bajte… Esc zastaví
bin-bytes = bajtov: { $count }
bin-same = Oba súbory sú rovnaké, bajt po bajte: každý má { $bytes } bajtov
bin-where = rozdiel { $at } z { $count } · prvý na pozícii { $offset } · líšiacich sa bajtov: { $differing }
bin-hint = Alt+↑↓ rozdiely · ↑↓ PgUp PgDn Home End riadky · Esc zatvoriť
diff-unified-on = Jeden stĺpec: čo odišlo, potom čo prišlo
diff-unified-off = Dva stĺpce vedľa seba
diff-patch-title = Uložiť rozdiely ako patch (.diff)
diff-patch-saved = { $name } zapísaný: zjednotený diff, riadkov: { $lines }
diff-patch-nothing = Oba súbory sú rovnaké: nie je patch na uloženie
# wastebasket and external editor
drive-trash = Kôš
drive-trash-closed = Kôš (tento systém nedovolí programu vypísať ho)
menu-trash = Kôš
menu-trash-empty = Vysypať Kôš
key-trash-restore = Obnov
key-trash-empty = Vysyp
btn-empty = Vysypať
btn-rename = Premenovať
trash-closed = Kôš tohto systému sa dá otvoriť len z jeho vlastného správcu súborov
trash-empty = Kôš je prázdny
trash-failed = Kôš: { $why }
trash-restored =
    { $count ->
        [one] jedna položka späť, kde bola
        [few] { $count } položky späť, kde boli
       *[other] { $count } položiek späť, kde boli
    }
trash-purged =
    { $count ->
        [one] jedna položka zmazaná natrvalo
        [few] { $count } položky zmazané natrvalo
       *[other] { $count } položiek zmazaných natrvalo
    }
trash-card-name = Názov
trash-card-from = Bol v
trash-card-deleted = Zmazané
trash-card-size = Veľkosť
trash-clash-title = Obnoviť
trash-clash-body = { $path } už existuje.
trash-clash-note = Prepísať pošle existujúci do Koša; Premenovať vráti tento vedľa neho.
trash-clash-more =
    { $count ->
        [one] Ešte jedna položka je v rovnakej situácii.
        [few] Ešte { $count } položky sú v rovnakej situácii.
       *[other] Ešte { $count } položiek je v rovnakej situácii.
    }
trash-purge-title = Zmazať natrvalo
trash-purge-body =
    { $count ->
        [one] Chcete zmazať { $name } z Koša?
        [few] Chcete zmazať { $count } položky z Koša?
       *[other] Chcete zmazať { $count } položiek z Koša?
    }
trash-purge-note = Toto maže natrvalo: nič ich už nevráti.
trash-empty-title = Vysypať Kôš
trash-empty-body = Zmazať všetko v Koši?
opt-editor-external = Externý editor
opt-editor-use-external = Použiť externý editor pre F4 (Alt+F4 potom otvára vstavaný)
opt-viewer-external = Externý zobrazovač
opt-viewer-use-external = Použiť externý zobrazovač pre F3 (Alt+F3 potom otvára vstavaný)


# editor features
opt-editor-line-numbers = Číslovať riadky
editor-line-numbers-on = Čísla riadkov zapnuté
editor-line-numbers-off = Čísla riadkov vypnuté
editor-prompt-pipe-block = Prehnať blok cez príkaz:
editor-prompt-pipe-file = Prehnať celý súbor cez príkaz:
editor-prompt-insert-output = Vložiť, čo tento príkaz vypíše:
editor-prompt-insert-file = Vložiť súbor:
editor-prompt-write-block = Zapísať blok do súboru:
editor-pipe-running = Spúšťam { $command } …
editor-pipe-done = { $command }: nahradené tým, čo vypísal, riadkov: { $lines } (Ctrl+Z to vráti)
editor-pipe-inserted = { $command }: vložených riadkov: { $lines }
editor-pipe-empty = { $command } nič nevypísal: nič sa nevložilo
editor-pipe-failed = { $command }: { $error } — nič sa nezmenilo
editor-pipe-changed = { $command }: text sa počas behu príkazu zmenil — nič sa nenahradilo
editor-pipe-gone = { $command }: súbor, pre ktorý bežal, už nie je otvorený — nič sa nezmenilo
editor-pipe-column = Stĺpcový blok sa nedá prehnať cez príkaz: označte celé riadky, alebo nič pre celý súbor
editor-file-inserted = { $name } vložený, riadkov: { $lines }
editor-insert-not-local = Do editora sa dá načítať a z neho zapísať len súbor na tomto počítači
editor-insert-too-big = priveľký na načítanie do editora
editor-block-written = Blok zapísaný do { $name }: { $size }
editor-block-write-self = { $name } je práve upravovaný súbor: blok sa cezeň nezapísal
editor-choice-hint = Píšte na zúženie · Enter: vybrať · Esc: späť
editor-syntax-title = Farbiť súbor ako
editor-syntax-auto = Čo hovorí jeho názov: { $language }
editor-syntax-auto-none = Čo hovorí jeho názov: nič známe
editor-syntax-none = Čistý text, bez farieb
editor-syntax-set = { $name } sa teraz číta ako { $language }
editor-syntax-off = { $name } sa teraz číta ako čistý text
editor-case-heading = Zmeniť veľkosť písmen
editor-case-upper = VEĽKÉ PÍSMENÁ
editor-case-lower = malé písmená
editor-case-title = Veľké Začiatočné: veľké písmeno na začiatku každého slova
editor-case-toggle = pREPNÚŤ: veľké sa stanú malými a malé veľkými
editor-no-word = Nič nie je označené a kurzor nie je na slove
editor-prompt-isearch = Hľadanie počas písania (↓ ↑ ďalší a predchádzajúci, Esc sa vráti)
editor-word-only = Slovo pod kurzorom nie je napísané nikde inde
editor-not-on-word = Kurzor nie je na slove
editor-prompt-reflow = Na koľko stĺpcov preformátovať odsek? (samotný Enter: { $width })
editor-reflowed = Odsek preformátovaný na { $width } stĺpcov (Ctrl+Z to vráti)
editor-reflow-bad-width = { $width } nie je šírka: počet stĺpcov od 8 do 1000
editor-no-paragraph = Kurzor je na prázdnom riadku: nie je tu odsek
editor-no-name-yet = Tento súbor ešte nemá názov: najprv ho uložte
editor-tools-title = Príkazy editora
editor-tool-isearch = Hľadanie počas písania
editor-tool-word-next = Ďalšie miesto, kde je slovo pod kurzorom
editor-tool-word-prev = Predchádzajúce miesto, kde je slovo pod kurzorom
editor-tool-pipe = Prehnať blok alebo celý súbor cez príkaz
editor-tool-write-block = Zapísať blok do súboru
editor-tool-case = Zmeniť veľkosť písmen bloku
editor-tool-reflow = Preformátovať odsek na šírku
editor-tool-insert-file = Vložiť súbor na kurzor
editor-tool-insert-output = Vložiť, čo vypíše príkaz
editor-tool-insert-date = Vložiť dátum a čas
editor-tool-insert-name = Vložiť názov tohto súboru
editor-tool-insert-path = Vložiť celú cestu tohto súboru
editor-tool-line-numbers = Čísla riadkov zapnúť alebo vypnúť
editor-tool-syntax = Vybrať jazyk, podľa ktorého sa súbor farbí
