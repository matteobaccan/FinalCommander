# Key bar labels: short, they must fit in a few columns.
key-help = Pagalb
key-menu = Meniu
key-view = Žiūrėt
key-edit = Redag
key-copy = Kopij
key-print = Spausd
key-move = Perkel
key-mkdir = Katal
key-delete = Trinti
key-pulldown = KonfMn
key-quit = Išeiti
# The key bar while a modifier is held: FAR's own words, six columns wide.
key-add = Supak
key-extract = Išpak
key-arccmd = ArcKmd
key-edit-new = Reda…
key-rename = Pervad
key-group = Grupė
key-selup = ŽymAuk
key-left = Kairė
key-right = Dešinė
key-sort-name = Vardą
key-sort-ext = Plėtin
key-sort-write = Modif
key-sort-size = Dydį
key-sort-unsort = Nerūš
key-sort-create = Sukūr
key-sort-access = Naudoj
key-sort = Rūšiuot
key-mklink = Saitas
key-find = Rasti
key-history = Istor
key-folders = KatIs
key-plugins = Pried

# Panel chrome
list-up = Aukštyn
list-folder = Katalogas
col-name = Vardas
col-ext = Plėt
col-size = Dydis
col-allocated = Užimta
col-modified = Modifikuotas
col-created = Sukurtas
col-accessed = Naudotas
col-attr = Atrib
col-owner = Savininkas
col-group = Grupė
col-descr = Aprašymas
col-links = Sts
col-git = Git
descr-title = { $name } aprašymas
descr-set = { $name } aprašytas
descr-cleared = { $name } aprašymas pašalintas
descr-not-here = aprašymai laikomi šalia katalogo šiame kompiuteryje
col-date-time = Data       Laikas
panel-list-count = elementų sąraše: { $count }
panel-empty = (tuščia)
panel-loading = skaitoma…

# Prompts
prompt-select-mask = Pažymėti failus pagal maskuotę:
prompt-deselect-mask = Atžymėti failus pagal maskuotę:
prompt-goto = Pereiti į katalogą:
prompt-filter = Failo maskuotė arba vardo dalis:
prompt-search = Greita paieška:

# Dialogs
dialog-error = Klaida
dialog-help-hint = Esc grįžta į turinį · F7 ieško visuose skyriuose · F1 uždaro · ↑↓ PgUp PgDn slenka
dialog-help-index = FinalCommander { $version } — Pagalba
dialog-help-hint-index = Enter atveria skyrių · F7 ieško visuose skyriuose · Esc uždaro
help-search-prompt = Ieškoti pagalboje
help-search-none = Nė vienoje pagalbos eilutėje nėra „{ $query }“
help-search-title = „{ $query }“ pagalboje: eilučių { $count }
output-search-prompt = Ieškoti išvestyje
output-search-empty = Dar nieko neišvesta
output-search-found = Eilutė { $line } iš { $lines }
output-search-wrapped = Eilutė { $line } iš { $lines }, vėl nuo naujausios
output-search-none = Nė vienoje išvesties eilutėje nėra „{ $query }“

# Status messages
msg-not-implemented = { $command } šioje versijoje dar neįgyvendinta
msg-selected = pažymėta elementų: { $count }
msg-deselected = atžymėta elementų: { $count }
msg-mask-invalid = neteisinga maskuotė: { $error }
msg-theme = tema: { $name }
msg-font-bigger = šriftas: terminalo paprašyta didesnio
msg-font-smaller = šriftas: terminalo paprašyta mažesnio
msg-font-reset = šriftas: terminalo paprašyta pradinio
msg-sort = rūšiavimas: { $key }{ $reverse }
msg-sort-groups-on = rūšiavimo grupės įjungtos ({ $count })
msg-sort-groups-off = rūšiavimo grupės išjungtos ({ $count })
msg-sort-groups-none = rūšiavimo grupių neapibrėžta: žr. [[sort_group]] konfigūracijoje
msg-view = vaizdas: { $mode }
msg-hidden-on = paslėpti failai rodomi
msg-hidden-off = paslėpti failai nerodomi
msg-no-history = istorijoje daugiau nieko nėra
msg-tab-last = langas visada turi vieną kortelę
msg-tab-end = kortelė jau yra tame eilės gale
tabs-title = Kortelės
msg-tab-moved = kortelė perkelta į kitą langą
msg-tab-locked = ši kortelė užrakinta savo kataloge
msg-tab-locked-on = kortelė užrakinta savo kataloge
msg-tab-locked-off = kortelė atrakinta
msg-bookmark-set = nuoroda { $digit }: { $path }
msg-bookmark-unset = nuoroda { $digit } nenustatyta
msg-bookmark-added = { $path } įsimintas
msg-bookmark-removed = { $path } pašalintas
msg-bookmark-known = šis katalogas jau yra sąraše
msg-dirsize = { $path }: { $bytes }, failų: { $files }
msg-dirsize-many = matuojami katalogai ({ $count })…
usage-title = Disko naudojimas: { $path } · { $size } · Tab: visas medis kaip žemėlapis
usage-not-here = katalogo disko naudojimas matuojamas diske, ne archyve ir ne rastų failų sąraše
msg-dirsize-no-folder = čia nėra katalogo, kurį būtų galima išmatuoti

# Jobs: prompts, confirmations, progress, conflicts
prompt-copy-to = Kopijuoti { $what } į:
prompt-move-to = Pervadinti ar perkelti { $what } į:
prompt-mkdir = Sukurti katalogą:
prompt-tab-name = Šios kortelės pavadinimas:
prompt-bookmark-name = Šio katalogo pavadinimas:
what-selected = pažymėtus elementus ({ $count })
confirm-delete-title = Trinti
confirm-delete-body =
    { $count ->
        [1] Ar norite ištrinti { $name } ({ $bytes })?
        [one] Ar norite ištrinti { $count } elementą ({ $bytes })?
        [few] Ar norite ištrinti { $count } elementus ({ $bytes })?
       *[other] Ar norite ištrinti { $count } elementų ({ $bytes })?
    }
prompt-link = Sukurti saitą į { $name } vardu:
link-created = { $name } sukurtas
confirm-wipe-title = Išvalyti
confirm-wipe-body =
    { $count ->
        [1] Perrašyti ir ištrinti { $name } ({ $bytes })?
        [one] Perrašyti ir ištrinti { $count } elementą ({ $bytes })?
        [few] Perrašyti ir ištrinti { $count } elementus ({ $bytes })?
       *[other] Perrašyti ir ištrinti { $count } elementų ({ $bytes })?
    }
confirm-wipe-honest = Baitai perrašomi vieną kartą, tada failas pašalinamas. SSD diske, „copy-on-write“ failų sistemoje ar esant momentinėms kopijoms senesnės kopijos gali išlikti — o vardas lieka kataloge ir žurnale.
job-copy = Kopijuoju
job-move = Perkeliu
job-delete = Trinu
job-wipe = Išvalau
job-trash = Perkeliu į šiukšlinę
job-replace = Keičiu
job-pack = Pakuoju
job-rebuild = Rašau archyvą

# ── the attributes dialog (SPEC 10 §3.8) ───────────────────────────────────────────────
attr-title = Failo atributai
attr-count = pažymėta elementų: { $count }
attr-now = dabar { $text } ({ $octal })
attr-owner-read = Savininkas gali skaityti
attr-owner-write = Savininkas gali rašyti
attr-owner-exec = Savininkas gali vykdyti
attr-group-read = Grupė gali skaityti
attr-group-write = Grupė gali rašyti
attr-group-exec = Grupė gali vykdyti
attr-other-read = Visi gali skaityti
attr-other-write = Visi gali rašyti
attr-other-exec = Visi gali vykdyti
attr-setuid = Vykdyti savininko vardu (setuid)
attr-setgid = Vykdyti grupės vardu (setgid)
attr-sticky = Trinti gali tik savininkas (sticky)
attr-readonly = Tik skaitymui
attr-hidden = Paslėptas
attr-system = Sisteminis
attr-archive = Archyvinis
attr-temporary = Laikinas
attr-offline = Neprisijungęs
attr-indexed = Indeksavimas leidžiamas
attr-compressed = suglaudintas
attr-encrypted = užšifruotas
attr-sparse = išretintas
attr-reparse = „reparse point“
attr-also = taip pat { $what }
attr-mtime = Modifikuotas
attr-atime = Paskutinį kartą skaitytas
attr-btime = Sukurtas
attr-ctime = Įrašas pakeistas
attr-btime-readonly = (sukūrimo laiką galima nustatyti tik Windows sistemoje)
attr-utc = Laikai UTC
attr-owner = Savininkas (vartotojas:grupė)
attr-owner-is = savininkas { $who }
btn-attr-original = Pradinis
btn-attr-current = Einamas
btn-attr-blank = Tuščias
attr-recursive = Ir viskam, kas yra kataloguose
attr-bad-time = `{ $text }` nėra data ir laikas (MMMM-MM-DD VV:MM:SS)
attr-bad-owner = čia nėra paskyros ar grupės vardu `{ $text }`
attr-in-archive = šis pakeitimas perrašys archyvą, visi kiti įrašai liks tokie, kokie yra
job-attrs = Uždedu atributus

# ── the copy dialog (SPEC 03 §4) ───────────────────────────────────────────────────────
copy-title = Kopijuoti
copy-title-move = Pervadinti/Perkelti
copy-target = Į
copy-conflict = Jei toks failas jau yra
copy-conflict-ask = klausti
copy-conflict-overwrite = perrašyti
copy-conflict-newer = perrašyti, jei šaltinis naujesnis
copy-conflict-skip = praleisti
copy-conflict-rename = duoti kopijai laisvą vardą
copy-conflict-append = prijungti prie galo
copy-only-newer = Tik naujesnius failus nei ten esantys
copy-filter = Tik atitinkančius vardus
copy-follow-links = Kopijuoti tai, į ką rodo saitas, o ne saitą
copy-keep-times = Išsaugoti laiko žymas
copy-keep-mode = Išsaugoti priėjimo teises
copy-verify = Kiekvieną failą perskaityti iš naujo ir palyginti
copy-several = paskirties vietų: { $count }, viena po kitos

# ── the terminal (SPEC 06 §4) ──────────────────────────────────────────────────────────
term-started = terminalas, kuriame veikia { $shell }
term-hint = Ctrl+Alt+E: atgal į langus
term-ended = baigėsi — bet kuris klavišas uždaro
job-mkdir = Kuriu katalogą
job-planning = planuoju…
job-counters = failų { $files } / { $filestotal } · { $bytes } / { $bytestotal }
job-rates = { $speed }/s · praėjo { $elapsed } · liko { $eta }
job-rates-no-eta = { $speed }/s · praėjo { $elapsed }
job-hint = Esc atšaukia
job-hint-queue = Esc atšaukia · B į foną
job-queued = { $kind } laukia eilėje, nebaigtų užduočių prieš ją: { $ahead } · Alt+Shift+F5 jas parodo
job-background = { $kind } tęsiama fone · Alt+Shift+F5 parodo užduotis
job-asks = { $kind } laukia atsakymo · Alt+Shift+F5
job-paused = { $kind }: pristabdyta
job-resumed = { $kind }: tęsiama
job-retry-nothing = Nėra ko kartoti
job-still-waiting = Ši užduotis laukia savo eilės
jobs-title = Užduotys (nebaigtų: { $active })
jobs-hint = Enter rodyti · Space pauzė · Del stabdyti · F5 kartoti · Ctrl+↑↓ tvarka · Esc uždaryti
jobs-quit-title = Užduotys dar vykdomos
jobs-quit-body = Šios užduotys sustos, jei programa dabar bus baigta:
job-state-running = vykdoma
job-state-paused = pristabdyta
job-state-asking = klausia
job-state-queued = laukia
job-state-held = sulaikyta
job-state-done = baigta
job-state-failed = klaidos
job-state-cancelled = atšaukta
job-done = { $kind }: failų { $files }, { $bytes }
job-done-renamed = { $kind }: be kopijavimo perkelta failų: { $files }
job-done-skipped = , praleista: { $skipped }
job-done-verified = , patikrinta: { $verified }
job-done-unverified = , čia nepavyko perskaityti iš naujo: { $unverified }
job-done-errors = , klaidų: { $errors }
job-cancelled = { $kind }: atšaukta
job-errors-title = Klaidos
conflict-title = Toks failas jau yra
conflict-dst = esantis        { $path }   { $bytes }
conflict-src = kopijuojamas   { $path }   { $bytes }
conflict-newer = šaltinis naujesnis
conflict-older = paskirties failas naujesnis
btn-overwrite = Perrašyti
btn-overwrite-all = Visus
btn-skip = Praleisti
btn-skip-all = Praleisti visus
btn-retry = Bandyti vėl
btn-move = Perkelti
btn-delete = Trinti
btn-wipe = Išvalyti
btn-copy = Kopijuoti
btn-discard = Atmesti
btn-kill = Nutraukti
btn-print = Spausdinti
btn-quit = Išeiti
btn-play = Vykdyti
btn-run = Paleisti
btn-accept = Priimti
btn-cancel = Atšaukti
msg-nothing-to-do = nieko nepažymėta
cmd-complete-nothing = nėra kuo to užbaigti
complete-several = taip prasidedančių vardų: { $count }
form-moving = rodyklės stumdo dialogą · Enter arba Esc palieka jį ten
form-moved = dialogas lieka ten, kur jį padėjote
msg-trash-refused = Šiukšlinė to nepriėmė — Shift+Del ištrina visam laikui.

# Checksums (SPEC 03 §9)
sum-title = Kontrolinės sumos
sum-algorithm = Maiša
sum-one-file = Vienas sąrašas visiems
sum-name = Vardas
sum-working = { $name }: { $done } iš { $total }
sum-verifying = tikrinama pagal { $name }
sum-stopped = sustabdyta
sum-written = { $name } įrašytas
sum-no-name = sąrašui reikia vardo
sum-local-only = kontrolinė suma gali skaityti tik šio kompiuterio failą
print-title = Spausdinimas
print-body = Spausdinti {$what}?
print-sending = siunčiama į spausdintuvą…
print-sent = išsiųsta į spausdintuvą
print-failed = spausdinti nepavyko ({$line}): {$error}
print-local-only = spausdinti galima tik šio kompiuterio failą
print-no-command = nėra spausdinimo komandos: nustatykite [print] command arba įdiekite lp ar lpr
sum-all-match = visi { $count } sutampa
sum-verify-title = Kontrolinės sumos
sum-verify-summary = sutampa: { $ok }, nesutampa: { $bad }
sum-differs = nesutampa
sum-missing = nėra
sum-unreadable = nepavyksta perskaityti

# Synchronising two directories (SPEC 03 §10)
sync-title = Sinchronizuoti
sync-between = { $left }  ↔  { $right }
sync-recursive = Ir viską pakatalogiuose
sync-both-ways = Į abi puses, laimi naujesnis
sync-delete-extra = Siūlyti pašalinti tai, ko kita pusė neturi
sync-by-content = Lyginti baitus, o ne dydį ir laiką
sync-filter = Tik atitinkančius vardus
sync-walking = žiūrima { $at } — kol kas { $pairs }
sync-planned = kopijuoti: { $copies } ({ $bytes }), pašalinti: { $deletes }
sync-hint = Space kitas atsakymas · ← → kryptis · = palikti · F2 vykdyti · Esc atšaukti
sync-hint-walking = Esc sustabdo peržiūrą
sync-nothing-to-do = visos eilutės sako palikti viską, kaip yra
sync-local-only = sinchronizavimas gali skaityti ir rašyti tik šiame kompiuteryje
sync-same-directory = abu langai yra tame pačiame kataloge

# What to run for a kind of file (SPEC 06 §5)
assoc-none = niekas čia nenurodo, kaip atverti { $name }
usermenu-empty = Vartotojo meniu dar nėra: parašykite jį faile { $path }
usermenu-last-gone = paskutinį kartą pasirinktos eilutės šiame meniu nebėra
usermenu-bad-condition = Nesuprantama sąlyga — { $problem }
assoc-desktop = Perduoti darbalaukiui
assoc-open-with = Atverti { $name } su
assoc-no-pager = nei asociacija, nei $PAGER nenurodo, kuo tai skaityti
assoc-no-editor = nei asociacija, nei $VISUAL nenurodo, kuo tai redaguoti

# Viewer (F3)
key-wrap = Kelt
key-hex = Hex
key-text = Tekst
key-dump = Dump
key-doc = Dok
key-search = Rasti
key-code = Kodas
key-follow = Sekti
viewer-prompt-search = Ieškoti ({ $mode }):
viewer-prompt-goto = Eiti į poslinkį arba NN% :
viewer-not-found = `{ $query }` nerasta
viewer-follow-on = sekama failo pabaiga
viewer-follow-off = sekimas išjungtas
viewer-codepage = koduotė: { $name }
viewer-wrap-mode = { $mode }
viewer-no-undo = ankstesnės pozicijos nėra
viewer-empty = (tuščias failas)
viewer-not-markdown = tai ne Markdown failas arba jis per didelis išdėstyti
viewer-not-table = tai ne lentelinis failas arba jis per didelis perskaityti visą
viewer-base64-done = Dekoduota: { $bytes }
viewer-not-base64 = Šis failas nėra base64: { $why }
viewer-status = { $mode } { $codepage }  { $offset } / { $len }  { $percent }%

key-disasm = Disasm
viewer-disasm-failed = nepavyksta disasembliuoti { $name }: { $error }
viewer-decompile-running = { $tool } apdoroja { $name }…
viewer-decompile-none = čia niekas negali dekompiliuoti { $format }; įdiekite įrankį ir pridėkite [[decompile]] taisyklę į fcmd.toml
arc-needs-program = šį archyvą skaito `{ $program }`, kuri šiame kompiuteryje neįdiegta
viewer-decompile-failed = { $tool }: { $error }
viewer-blame-derived = git blame taikomas pačiam failui: pirmiausia grįžkite į jį
viewer-derived = { $producer }: { $name }

# Editor (F4)
key-save = Išsaug
key-replace = Keisti
editor-prompt-goto = Eiti į eilutę arba eilutė:stulpelis :
editor-prompt-save-as = Išsaugoti failą vardu:
editor-saved = { $name }: išsaugota, baitų: { $bytes }
editor-nothing-to-save = { $name } neturi neišsaugotų pakeitimų
editor-not-found = `{ $query }` nerasta
editor-replaced = pakeitimų: { $count }
editor-read-only = { $name } yra tik skaitymui
editor-readonly-on = tik skaitymui: redagavimas atmetamas
editor-readonly-off = tik skaitymo režimas išjungtas
editor-no-undo = nebėra ko atšaukti
editor-no-redo = nėra ko pakartoti
editor-discard-title = Neišsaugoti pakeitimai
editor-discard-body = { $name } turi neišsaugotų pakeitimų. Išeiti ir juos prarasti?
editor-changed-title = Pakeistas diske
editor-changed-body = { $name } pasikeitė diske nuo tada, kai buvo atvertas. Perrašyti jį?
editor-reloaded = { $name } iš naujo įkeltas iš disko
editor-failed = { $name }: { $error }

# netbox (SPEC 04 §6)
net-sites-title = Prisijungimai
net-sites-hint = Enter jungtis · n URL · Ins įsiminti · F4 redaguoti · d atsijungti · Esc uždaryti
net-no-sites = Serverių dar nėra. Spauskite `n` ir įveskite URL arba pridėkite [[site]] blokus į konfigūraciją.
net-not-a-stored-session = ši eilutė yra atviras prisijungimas, o ne išsaugota sesija
net-site-forgotten = { $name } nebėra išsaugota
net-prompt-url = Jungtis prie:
net-prompt-password = { $user }@{ $host } slaptažodis:
net-prompt-passphrase = { $user }@{ $host } rakto slaptafrazė:
net-connecting = jungiamasi prie { $host }…
net-connected = prisijungta prie { $host }
net-disconnected = { $host } atjungtas
net-not-connected = { $host } neprijungtas
net-local-panel = šis langas yra vietinėje failų sistemoje
net-nothing-pending = joks prisijungimas to nelaukia
net-hostkey-title = Nežinomas serverio raktas
net-hostkey-body = { $host } nėra faile known_hosts. Jo piršto atspaudas:
net-hostkey-question = Įrašyti jį ir jungtis?
net-hostkey-changed-title = Serverio raktas PASIKEITĖ
net-hostkey-changed-body = { $host } atsakė raktu, kuris nesutampa su įrašytuoju.
net-hostkey-changed-where = Įrašytasis yra { $path }, eilutė { $line }.
net-hostkey-changed-what = Perdiegtas serveris atrodo lygiai taip pat, kaip kažkas įsiterpęs per vidurį. Čia niekas nepriimama: atverkite failą, perskaitykite eilutę ir pašalinkite ją patys, jei žinote, kodėl ji pasikeitė.
btn-open-file = Atverti failą
net-fetching = atsiunčiamas { $name }…
net-viewing-copy = { $name }: vietinė kopija, atsiųsta peržiūrai
net-too-big = { $name } didesnis nei { $limit }: norėdami atsisiųsti, padidinkite net.fetch_limit_mb
net-editing = { $name }: redaguojama vietinė kopija; išsaugant ji įrašoma atgal
net-writing-back = { $name } rašomas atgal…
net-written-back = { $name }: įrašyta atgal, baitų: { $bytes }
net-write-back-failed = įrašyti atgal nepavyko: { $error } — jūsų pakeitimai yra { $local }
net-changed-title = Pakeistas serveryje
net-changed-body = { $name } pasikeitė serveryje nuo tada, kai buvo atsiųstas. Perrašyti jį?
confirm-delete-permanent = Tai ištrina visam laikui: šiems elementams šiukšlinės nėra.
confirm-trash-title = Perkelti į šiukšlinę
confirm-trash-body =
    { $count ->
        [1] Ar norite perkelti { $name } ({ $bytes }) į šiukšlinę?
        [one] Ar norite perkelti { $count } elementą ({ $bytes }) į šiukšlinę?
        [few] Ar norite perkelti { $count } elementus ({ $bytes }) į šiukšlinę?
       *[other] Ar norite perkelti { $count } elementų ({ $bytes }) į šiukšlinę?
    }
confirm-trash-where = Jie keliauja ten, kur darbalaukis laiko ištrintus failus, ir iš ten juos galima atkurti.

# ---------------------------------------------------------------- menu bar (SPEC 10)
menu-left = &Kairė
menu-right = &Dešinė
menu-files = &Failai
menu-commands = Ko&mandos
menu-options = &Opcijos
menu-brief = &Trumpai
menu-medium = &Vidutiniškai
menu-full = &Pilnai
menu-detailed = &Detaliai
menu-owners = &Failo savininkai
menu-links = Failo &saitai
menu-alt-full = Pilnai, tikslūs dyd&žiai
menu-descriptions = Apraš&ymai
menu-long-descriptions = Il&gi aprašymai
menu-wide = P&lačiai
menu-info-panel = &Info langas
menu-quick-view = Vai&zdas
menu-tree-panel = &Medis
menu-tab-new = &Nauja kortelė
menu-tab-close = Uždaryti kortelę
menu-tab-next = &Kita kortelė
menu-tab-prev = Ankstesn&ė kortelė
menu-tab-move = Perkelti kort&elę į kitą langą
menu-tab-lock = &Užrakinti kortelę jos kataloge
menu-tab-name = Pavadinti šią kortelę
menu-tab-list = Visos kortelės
menu-sort-modes = Rūšiavimo b&ūdai
menu-hidden = Paslėpti ir sisteminiai failai
menu-filter = Failų filt&ras
menu-filter-sets = Pavadinti filtrai
menu-panel-onoff = Langą į&jungti/išjungti
menu-reread = &Atnaujinti
menu-change-drive = Keisti disk&ą
menu-view = &Žiūrėti
menu-edit = R&edaguoti
menu-edit-new = Redaguoti &naują failą
menu-copy = &Kopijuoti
menu-rename = Pe&rvadinti/Perkelti
menu-link = Sukurti sa&itą
menu-multirename = Pervadinti pagal šabloną…
menu-multirename-undo = Atšaukti paskutinį grupinį pervadinimą
menu-mkdir = &Sukurti katalogą
menu-delete = &Trinti
menu-wipe = Išva&lyti
menu-checksum = Kontrolinės s&umos…
menu-checksum-verify = Patikrinti pagal sąrašą
menu-split = Padalinti failą…
menu-combine = Sujungti failus
menu-delete-permanent = Trinti visa&m laikui
menu-archive-add = Su&pakuoti į archyvą
menu-archive-extract = Išar&chyvuoti failus
menu-archive-commands = Arc&hyvo komandos
menu-archive-test = Tikrinti archyvą
menu-attributes = Failo &atributai
menu-apply-command = Panaudoti k&omandą
menu-replace-in-files = Pakeisti failuose…
menu-select-group = Pažymėti &grupę
menu-select-ext = Pažymėti pagal šį plėtin&į
menu-unselect-ext = Atžymėti pagal &šį plėtinį
menu-unselect-group = Atžymėti grup&ę
menu-invert-selection = At&virkščiai pažymėti
menu-select-all = Pažymėti viską
menu-restore-selection = Atnau&jinti žymėjimą
menu-dirsize = Katalogo &dydis
menu-find-file = &Rasti failą
menu-history = Komandų &istorija
menu-view-history = Per&žiūros ir redagavimo istorija
menu-folders-history = Katal&ogų istorija
menu-usage = Disko naudo&jimas
menu-jobs = &Užduotys
menu-branch = Šakos vaizdas
menu-bookmarks = Katalogų nuorodos
menu-swap-panels = &Sukeisti langus
menu-sync-panel = Sulyginti kitą lan&gą
menu-panels-onoff = &Langus įjungti/išjungti
menu-compare-folders = &Palyginti katalogus
menu-sync-folders = Si&nchronizuoti katalogus
menu-compare-contents = Palyginti pagal turin&į
menu-panelize = Komandos išvestį į langą
menu-view-filtered = Žiūrėti per komandą
menu-learn-keys = Išmokti klavišus
menu-tunnel = Atidaryti &tunelį…
menu-tunnels = Tun&eliai…
tunnel-title = Tunelis: [vietinis:]serveris:prievadas
tunnel-list-title = Tuneliai
tunnel-none = nėra atidaryto tunelio
tunnel-open = tunelis atidarytas: { $what }
tunnel-closed = tunelis uždarytas
tunnel-failed = { $what }: { $why }
tunnel-needs-connection = tunelis eina per prisijungimą: pirmiausia atverkite jį su Alt+N
menu-connections = Prisijungimai
menu-disconnect = Atsijungti
menu-user-menu = &Vartotojo meniu
menu-associations = Failų &asociacijos
menu-folder-shortcuts = Katalogų nuoro&dos
menu-plugin-commands = Pried&ų komandos
menu-screens-list = E&kranų sąrašas
menu-macro-record = Įrašyti makrokomandą
menu-macro-list = Makrokomandos
menu-task-list = Užduo&čių sąrašas
menu-system-settings = Siste&miniai nustatymai
menu-panel-settings = La&ngų parametrai
menu-interface-settings = &Sąsajos nustatymai
menu-languages = Kal&bos
menu-themes = &Temos
menu-confirmations = &Patvirtinimai
menu-viewer-settings = Per&žiūros nustatymai
menu-editor-settings = &Redaktoriaus nustatymai
menu-codepages = &Koduotės
menu-sort-groups = Rūšiavimo &grupės
menu-mask-groups = &Failų maskuotės
menu-settings-all = &Visi nustatymai
menu-highlighting = Failų par&yškinimai
menu-plugins-config = Prie&dų konfigūracija
menu-help = Pagalba
menu-save-setup = &Išsaugoti nuostatus
menu-quit = Iš&eiti
opt-natural-sort = Skaitmenis laikyti skaičiais
opt-allow-reverse = Leisti atvirkštinį rūšiavimą
opt-select-folders = Maskuotės žymi ir katalogus
cmd-no-network-name = Kelias šiame diske neturi tinklo vardo
opt-trash = Trinti į šiukšlinę
opt-system-copy = Naudoti sisteminę kopijavimo funkciją
opt-save-history = Išsaugoti istoriją tarp seansų
opt-history-lifetime = Istorijos galiojimas dienomis
opt-forever = visam laikui
opt-fetch-limit = Nuotolinio failo dydžio riba
opt-keepalive = SSH keep-alive
opt-off = išjungta
opt-show-hidden = Rodyti paslėptus ir sisteminius failus
opt-column-titles = Rodyti stulpelių antraštes
opt-status-line = Rodyti būsenos eilutę
opt-scrollbar = Rodyti šliaužiklį
opt-free-space = Rodyti laisvos vietos kiekį
opt-auto-refresh = Automatiškai atnaujinti langą
opt-dirs-first = Pirmiau katalogai
opt-selected-first = Pirmiau pažymėti failai
opt-reverse = Atvirkštinė tvarka
opt-view = Lango vaizdas
opt-sort = Rūšiuoti pagal
opt-quick-search = Greita paieška
opt-quick-alt = su Alt
opt-quick-plain = raidėmis
opt-key-bar = Rodyti funkcinių klavišų juostą
opt-status-bar = Rodyti būsenos juostą
opt-syntax = Spalvinti atpažįstamas kalbas
opt-markdown = Markdown išdėstytas skaitymui
opt-table = Atverti .csv ar .jsonl kaip lentelę
opt-keymap = Klavišų išdėstymas
opt-clock = Laikrodis kampe
opt-mouse = Pelė
opt-clipboard = Leisti terminalui perduoti iškarpinę (OSC 52)
opt-command-screen = Rodyti komandos išvestį, kol ji vykdoma
opt-menu-always = Visada rodyti meniu juostą
opt-save-exit = Išeinant išsaugoti nuostatus
opt-confirm-delete = Trinant
opt-confirm-overwrite = Perrašant
opt-confirm-exit = Išeinant
opt-confirm-editor = Paliekant redaktorių su neišsaugotais pakeitimais
opt-applied = nustatymai pritaikyti
opt-saved = išsaugota į { $path }
opt-set-title = { $key }
opt-set = { $key } = { $value }
opt-not-a-toggle = Tai ne taip/ne nustatymas: Enter leidžia įvesti reikšmę
opt-settings-hint = ↑↓ rinktis · rašykite, kad susiaurintumėte · Enter arba F4 reikšmė · Space perjungti · Del numatytoji · Esc uždaryti
opt-language-set = kalba: { $name }
sort-name = Vardas
sort-ext = Plėtinys
sort-size = Dydis
sort-mtime = Modifikavimo laikas
sort-btime = Sukūrimo data
sort-atime = Naudojimo data
sort-descr = Aprašymas
sort-owner = Savininkas
sort-git = Git būsena
key-sort-descr = Aprašy
key-sort-owner = Savin
sort-none = Nerūšiuoti
sort-set = rūšiavimas: { $name }
btn-ok = Gerai
pick-hint = ↑↓ rinktis · rašykite, kad filtruotumėte · Enter taikyti · Esc uždaryti
pick-empty = (nėra iš ko rinktis)
# ------------------------------------------------- the three histories (SPEC 06 §6)
hist-pinned = { $line } prisegta · dabar niekas jos neišstums
hist-unpinned = { $line } nebeprisegta
hist-is-pinned = { $line } prisegta · Ins pirmiausia ją atsega
hist-clear-body = Išvalyti iš šios istorijos viską, kas neprisegta?
hist-cleared = istorija tuščia, liko tik tai, kas prisegta
history-hint = ↑↓ rinktis · Enter taikyti · Ins prisegti · Shift+Del vieną · Del visus · Esc uždaryti


# ------------------------------------------------- session editor (SPEC 10 §3.5)
site-new = Naujas prisijungimas
site-edit = Redaguoti prisijungimą
site-name = Vardas
site-host = Serveris
site-port = Prievadas
site-user = Vartotojas
link-title = Sukurti saitą
link-name = Saitas
link-kind = Rūšis
link-kind-relative = simbolinis, santykinis — išlieka perkėlus medį
link-kind-absolute = simbolinis, absoliutus — išlieka perkėlus saitą
link-kind-hard = kietasis — antras to paties failo vardas
menu-colors = Spa&lvos…
palette-title = Temos { $theme } spalvos
palette-hint = Enter spalva · F2 išsaugoti · Esc uždaryti
palette-ask = Spalva elementui { $key }
palette-bad-color = { $value } nėra spalva (pavadinimas, 0–255 arba #rrggbb)
palette-saved = spalvos įrašytos į temą { $theme }
palette-unsaved = spalvos pasikeitė tik ekrane — F2 jas įrašo
opt-viewer-status-line = Rodyti viršutinę eilutę (vardas ir pozicija)
opt-viewer-scrollbar = Rodyti šliaužiklį
opt-viewer-key-bar = Rodyti funkcinių klavišų juostą
opt-viewer-wrap = Ilgos eilutės
opt-viewer-wrap-off = nekelti
opt-viewer-wrap-char = kelti ties kraštu
opt-viewer-wrap-word = kelti ties žodžiu
opt-viewer-tab-width = Tabuliacijos plotis
opt-viewer-remember = Įsiminti kiekvieno failo poziciją ir koduotę
opt-editor-tabs = Tabuliacijos
opt-editor-tabs-keep = palikti, kaip yra
opt-editor-tabs-typed = išskleisti tai, kas įvedama
opt-editor-tabs-all = išskleisti visas atveriant failą
opt-editor-tab-width = Tabuliacijos plotis
opt-editor-auto-indent = Išlaikyti aukščiau esančios eilutės įtrauką
opt-editor-whitespace = Rodyti tarpus ir tabuliacijas
opt-editor-cursor-after = Žymeklis atsiduria už paieškos rasto teksto
opt-editor-beyond-eol = Žymeklis gali eiti už eilutės pabaigos
opt-editor-trim-trailing = Išsaugant pašalinti tarpus eilučių gale
editor-not-a-character = šis klavišas neturi simbolio, kurį būtų galima įterpti
paste-one-line =
    { $count ->
        [one] įklijuota tik pirma eilutė · dar { $count } eilutė praleista
        [few] įklijuota tik pirma eilutė · dar { $count } eilutės praleistos
       *[other] įklijuota tik pirma eilutė · dar { $count } eilučių praleista
    }
clip-names =
    { $count ->
        [one] { $count } vardas iškarpinėje
        [few] { $count } vardai iškarpinėje
       *[other] { $count } vardų iškarpinėje
    }
clip-paths =
    { $count ->
        [one] { $count } kelias iškarpinėje
        [few] { $count } keliai iškarpinėje
       *[other] { $count } kelių iškarpinėje
    }
clip-line = komandinė eilutė yra iškarpinėje
clip-selection = tai, kas buvo pažymėta, yra iškarpinėje
clip-off = iškarpinė išjungta — `ui.clipboard` nustatymuose
history-gone = { $name } ten nebėra
hl-new = Nauja spalvų grupė
hl-edit = Spalvų grupė
# ------------------------------------------------- the diff screen (SPEC 05 §3)
diff-title = Skirtumai
menu-diff = Rodyti &failų skirtumus
diff-needs-two = Pastatykite žymeklį ant failo kiekviename lange
diff-not-here = { $name } nėra failas šiame diske
diff-too-big = { $name } didesnis nei { $most }
diff-same = Abu failai vienodi
diff-where = fragmentas { $at } iš { $count } · eilutės -{ $removed } +{ $added }
diff-no-next = Paskutinis skirtumas
diff-no-previous = Pirmas skirtumas
diff-hint = Alt+↑↓ fragmentai · Tab pusė · F5 perimti · F2 įrašyti · U bendras · Shift+F2 pataisa · W C E nepaisyti · Esc
diff-hint-git = Alt+↑↓ fragmentai · S paruošti fragmentą · Tab pusė · F5 perimti · F2 įrašyti · Esc uždaryti
diff-hint-git-staged = Alt+↑↓ fragmentai · S išimti fragmentą · Tab pusė · F5 perimti · F2 įrašyti · Esc uždaryti
diff-stage-edited = skirtumuose yra neįrašytų suliejimų: pirma F2, tada S
diff-merged-left = Kairysis failas perėmė šį skirtumą
diff-merged-right = Dešinysis failas perėmė šį skirtumą
diff-no-hunk-here = Ekrano viršuje skirtumo nėra — Alt+↓ jį ten pastato
diff-nothing-to-write = Niekas nebuvo sulieta
diff-written = įrašyta: { $count }
diff-unwritten = Suliejimas neįrašytas — F2 jį įrašo, dar vienas Esc palieka neįrašytą
diff-ignore-space-on = Tarpų nepaisoma
diff-ignore-space-off = Tarpai vėl skaičiuojami
diff-ignore-case-on = Raidžių dydžio nepaisoma
diff-ignore-case-off = Raidžių dydis vėl skaičiuojamas
diff-ignore-eol-on = Eilučių pabaigų nepaisoma
diff-ignore-eol-off = Eilučių pabaigos vėl skaičiuojamos

grab-hint = ↑↓←→ judėti · Shift+↑↓←→ žymėti · Ctrl+A viską · Enter kopijuoti · Esc uždaryti
grab-copied = nukopijuota ekrano eilučių: { $rows }

# ------------------------------------------------- the code pages (SPEC 05 §1.12)
cp-title = Koduotės
cp-name-title = Kaip pavadinti šią koduotę
cp-set = Skaitoma kaip { $name }
menu-panel-names = Vardų koduotė…
names-title = Vardus skaityti kaip
names-system = Kaip juos pateikia sistema
names-set = Vardai skaitomi kaip { $name }
names-system-set = Vardai tokie, kokius pateikia sistema
names-not-here = Šioje sistemoje vardas nėra baitai, todėl kito būdo jį perskaityti nėra
cp-marked = F8 dabar pereina ir per { $name }
cp-unmarked = F8 per { $name } nebepereina
cp-hint = ↑↓ rinktis · Enter skaityti taip · Ins žymėti F8 · F4 pervadinti · Esc uždaryti

# ------------------------------------------------- the named mask groups (SPEC 02 §5b)
mg-title = Maskuočių grupė
mg-name = Vardas
mg-mask = Maskuotės
mg-hint = Ins prideda grupę, Del pašalina, F4 redaguoja · `·` žymi tris įtaisytąsias
mg-saved = <{ $name }> galioja; „Išsaugoti nuostatus“ ją įrašo
mg-removed = <{ $name }> pašalinta
mg-bad = <{ $name }>: { $problem }
mg-needs-name = vardas be tarpų, kablelių ir kampinių skliaustų
mg-needs-mask = grupė, kuri nieko neatstovauja, nieko neatitinka
mg-built-in = <{ $name }> yra viena iš trijų įtaisytųjų · F4 suteikia jai kitas maskuotes

sg-title = Rūšiavimo grupė
sg-name = Vardas
sg-mask = Maskuotės
sg-place = Kur
sg-place-front = prieš kitus
sg-place-back = po kitų
sg-hint = Ins prideda grupę, Del pašalina, F4 redaguoja, Ctrl+↑↓ perkelia
sg-saved = Grupė galioja; „Išsaugoti nuostatus“ ją įrašo
sg-removed = { $name } pašalinta
sg-bad = { $name }: { $problem }
sg-needs-mask = grupėje be maskuotės nieko nebūtų
hl-name = Vardas
hl-mask = Maskuotė ar maskuotės
hl-kind = Taikoma
hl-kind-both = failams ir katalogams
hl-kind-file = tik failams
hl-kind-dir = tik katalogams
hl-size = Dydis (1M-10M, -4K, 1G-)
hl-age = Amžius dienomis (<=1, >=30, 7-30)
hl-date-field = Amžius skaičiuojamas pagal
hl-attrs = Atributai (h r x s b, - reiškia „negali turėti“)
hl-fg = Teksto spalva (#d7875f, red, file.archive)
hl-bg = Fonas, užrašomas taip pat
hl-hint = ↑↓ rinktis · Enter/F4 redaguoti · Ins nauja · Del šalinti · Ctrl+↑↓ perkelti · Esc uždaryti
hl-saved = grupė galioja · Opcijos → Išsaugoti nuostatus įrašo ją į failą
hl-removed = { $name } pašalinta · Opcijos → Išsaugoti nuostatus įrašo tai į failą
hl-bad = { $name }: { $problem }
filter-off = Be filtro: visi vardai
filter-hint = ↑↓ rinktis · Enter naudoti · F4 redaguoti · Ins naujas · Del šalinti · Esc uždaryti
filter-new = Naujas filtras
filter-edit = Filtras
filter-name-hint = su vardu jis lieka filtrų meniu; be vardo panaudojamas vieną kartą
filter-applied = filtras įjungtas: { $name }
filter-cleared = filtras išjungtas
filter-saved = { $name } taikomas langui ir yra filtrų meniu · Opcijos → Išsaugoti nuostatus įrašo jį į failą
filter-removed = { $name } pašalintas · Opcijos → Išsaugoti nuostatus įrašo tai į failą
editor-no-block = nieko nepažymėta
editor-unique = pašalinta pasikartojančių eilučių: { $count }
editor-no-bracket = čia nėra skliausto arba niekas jo neatitinka
editor-tabs-expanded = tabuliacijos išskleistos { $lines } eilutėse — išsaugant tai bus įrašyta
site-protocol = Protokolas
site-protocol-sftp = SFTP (per SSH)
site-protocol-ftp = FTP
site-protocol-ftps = FTP su TLS
site-protocol-dav = WebDAV
site-protocol-davs = WebDAV per HTTPS
site-auth = Tapatybės nustatymas
site-auth-agent = SSH agentas
site-auth-key = rakto failas
site-auth-ask = klausti slaptažodžio
site-key-file = Rakto failas
site-start-dir = Nuotolinis katalogas
site-group = Grupė
site-keepalive = Keep-alive (s)
site-compress = Glaudinimas
site-page-session = Sesija
site-page-environment = Aplinka
site-page-directories = Katalogai
site-page-sftp = SFTP
site-password-note = Slaptažodis nesaugomas: jo paprašoma jungiantis.
site-env-eol = Versti eilučių pabaigas (dar ne)
site-env-utf8 = Priverstinai UTF-8 vardai (dar ne)
site-jump = Tarpinis serveris
site-jump-note = Pasiekiama per šį serverį: host, user@host arba user@host:port. Tuščia reiškia tiesiai į serverį.
site-sftp-ciphers = Pasirinkti šifrus (dar ne)
btn-connect = Jungtis
site-needs-host = prisijungimui reikia serverio
site-needs-key = pasirinkite rakto failą arba kitą tapatybės nustatymo būdą
site-bad-port = `{ $port }` nėra skaičius
site-stored = prisijungimas išsaugotas: { $name } (Shift+F9 jį išlaikys)
# ------------------------------------------------- structure pane (SPEC 05 §1.8)
key-struct = Strukt
key-markdown = MD
key-table = Lentel
key-filter = Filtr
key-decompile = Dekomp
key-next = Kitas
key-prev = Ankst
key-codepages = Koduot
key-goto = Eiti
key-to-panel = Langas
key-blame = Blame
viewer-structure = Struktūra
viewer-structure-hint = ↑↓ eilutė · Enter eiti į · Tab turinys · Shift+F11 uždaryti
viewer-structure-hint-tab = Tab perkelia klavišus čia
viewer-no-structure = nėra ko dekoduoti: šio formato struktūros skydelis nepažįsta
viewer-no-target = ši eilutė niekur faile nerodo
viewer-form-opened = Forma kaip tekstas, kurį jai rodo Delphi. F4 grąžina prie baitų.
viewer-metadata-opened = Visas tekstas, išdėstytas skaitymui. F4 grąžina prie baitų.
viewer-went-to = pereita į { $offset }
viewer-in-panel = langas pastatytas ant { $name }
viewer-not-in-panel = { $name } šiame lange nėra
viewer-no-more-files = lange daugiau failų nėra
editor-unsaved-for-viewer = pirmiausia išsaugokite: peržiūra skaito failą iš disko
# ------------------------------------------------- command line (SPEC 06 §7)
cmd-running = vykdoma: { $command }
prompt-panelize = Komanda, kurios išvestis įvardija failus
prompt-viewer-filter = Komanda, kurios išvestį skaityti vietoj failo
learn-title = Kokius klavišus siunčia šis terminalas
learn-hint = Spauskite klavišus. Užsidegęs langelis reiškia, kad klavišas gautas; du kartus Esc — išeiti.
learn-last = paskutinis: { $key } — { $command }
learn-unbound = niekam nepriskirtas
recode-title = Perrašyti kaip
recode-what = failų: { $count }
recode-from = Kas jie dabar
recode-to = Rašyti juos kaip
recode-auto = nustatyti kiekvienam failui
recode-eol = Eilučių pabaigos
recode-eol-keep = palikti, kaip yra
recode-working = perrašomi failai: { $count }
recode-done = įrašyta: { $changed }, jau buvo tokie: { $same }, atmesta: { $failed }
menu-recode = Perrašyti kaip…
panelize-title = { $command }
panelize-done = failų: { $count }, praleista eilučių: { $dropped }
panelize-nothing = { $command }: nė viena eilutė neįvardijo esamo failo
cmd-interrupted = komandai nusiųstas pertraukimas
cmd-done = { $command } baigta
cmd-failed = { $command } baigėsi kodu { $code }
cmd-busy = komanda jau vykdoma
prompt-apply = Komanda, vykdoma kiekvienam iš { $count } failų:
cmd-apply-no-ask = komandos, kuri prašo reikšmės, negalima taikyti failas po failo
cmd-apply-progress = { $command }  ({ $done } iš { $total })
cmd-apply-stopped = sustabdyta po { $done } iš { $total }: { $command } nepavyko
cmd-not-remote = komandai reikia vietinio katalogo; šis langas yra serveryje
cmd-no-history = komandų istorija tuščia
cmd-no-previous = nėra kur grįžti
cmd-no-such-dir = { $path }: tokio katalogo nėra
cmd-no-such-path = { $path }: tokio failo ar katalogo nėra
cmd-no-selection = nieko nepažymėta
cmd-condition-false = nevykdyta: { $command }
cmd-screen-cleared = ekranas išvalytas
cmd-stack-empty = katalogų dėklas tuščias
cmd-stack-cleared = katalogų dėklas išvalytas
cmd-pushed = katalogų dėkle: { $count }
cmd-environment = aplinka
cmd-set = { $name } = { $value }
cmd-not-set = { $name } nenustatytas
user-screen-title = Išvestis — eilučių: { $lines }, nuo eilutės { $at }
user-screen-empty = (dar niekas nebuvo vykdyta)
viewer-no-branch = ši eilutė neturi adreso, kuriuo būtų galima sekti
viewer-no-back = nėra prie ko grįžti

# ------------------------------------------------- find files (SPEC 06 §1)
folders-title = Rasti katalogą
folders-reading = Skaitomi katalogai…
folders-read = katalogų: { $count }
folders-capped = katalogų: { $count }, iki ribos
folders-local-only = katalogų sąrašas yra šio kompiuterio; serveris turi savo istoriją ir nuorodų sąrašą
find-title = Rasti failą
find-mask = Failo maskuotė ar kelios maskuotės
find-size = Dydis (100M, 1M-1G, -4K)
find-text = Su tekstu
find-case = Didžiosios skiriasi
find-words = Tik visi žodžiai
find-hex = Aukščiau esantis tekstas yra šešioliktainis šablonas
find-not = Failai, kuriuose jo NĖRA
find-scope = Ieškoti
find-scope-panel = tik einamame kataloge
find-scope-tree = nuo einamo katalogo
find-scope-selection = pažymėtuose kataloguose
find-scope-root = nuo šio disko šakninio katalogo
find-scope-other = nuo kito lango katalogo
find-links = Ieškoti simboliniuose saituose
find-dirs = Ieškoti ir katalogų
find-archives = Ieškoti archyvuose
find-results = Rasta { $count } · peržiūrėta failų { $files } ir katalogų { $dirs }
find-done = rasta { $count }, failų { $files } · { $elapsed }
find-none = nieko nerasta
find-stopped = sustabdyta po { $files } failų
find-hint = ↑↓ rinktis · Enter eiti į · F3 žiūrėti · F4 redaguoti · F5 į langą · Esc stabdyti
find-hint-done = ↑↓ rinktis · Enter eiti į · F3 žiūrėti · F4 redaguoti · F5 į langą · Esc uždaryti
find-panelised = rezultatų lange: { $count }
find-needs-mask = reikia maskuotės: `*` reiškia visus failus
find-bad-size = `{ $text }` nėra dydis — { $error }

# ------------------------------------------------- replace across files
replace-title = Pakeisti failuose
replace-needle = Ko ieškoti
replace-with = Pakeisti į
replace-case = Didžiosios skiriasi
replace-words = Tik visi žodžiai
replace-pattern = Aukščiau esantis tekstas yra reguliarusis reiškinys
replace-hex = Abu yra šešioliktainiai baitai
replace-scope = pažymėta failų: { $count }
replace-needs-needle = nėra ko ieškoti
replace-confirm-title = Pakeisti
replace-confirm-body = Pasikeistų atitikmenų: { $count }, failuose: { $files }.
replace-nothing = nėra ko keisti: nė vienas failas nepasikeistų
replace-done = pakeista atitikmenų: { $count }, failuose: { $files }

# ── archives (SPEC 04 §4) ───────────────────────────────────────────────────────────────
arc-writing-back = { $name } rašomas atgal į archyvą
arc-move-out = norėdami ką nors išimti iš archyvo, nukopijuokite tai su F5
arc-add-files-only = po vieną dedami tik failai; Shift+F1 įdeda visą katalogą

rename-title = Grupinis pervadinimas
rename-name = Vardas
rename-ext = Plėtinys
rename-search = Ko ieškoti
rename-replace = Pakeisti į
rename-regex = Reguliarusis reiškinys ($1 pakaite)
rename-case = Raidės
rename-case-same = kaip yra
rename-case-lower = mažosios raidės
rename-case-upper = DIDŽIOSIOS RAIDĖS
rename-case-title = Kiekvienas Žodis Iš Didžiosios
rename-case-sentence = Tik pirma raidė didžioji
rename-counter = Skaitiklis: pradžia, žingsnis, skaitmenys
rename-fields = [N] vardas  [E] plėtinys  [N1-3] dalis  [C] skaitiklis  [d] [t] data, laikas  [P] katalogas
rename-not-in-archive = keli failai pervadinami kataloge, ne archyve: ten F6 pervadina vieną įrašą
rename-bad-counter = skaitiklis yra trys sveikieji skaičiai — pradžia, žingsnis, skaitmenys: { $counter } toks nėra
rename-cannot = pervadinti negalima: { $why }
rename-problem-empty = neliko vardo
rename-problem-invalid = tokio vardo failas turėti negali
rename-problem-twice = šį vardą gauna ir kitas failas
rename-problem-exists = failas tokiu vardu jau yra
rename-more = … ir dar { $count }
rename-blocked =
    { $count ->
        [one] { $count } failas negali gauti naujo vardo: kol negali, niekas nepervadinama
        [few] { $count } failai negali gauti naujų vardų: kol negali, niekas nepervadinama
       *[other] { $count } failų negali gauti naujų vardų: kol negali, niekas nepervadinama
    }
rename-summary = keičiasi { $count } iš { $total }
rename-nothing = nė vienas vardas nesikeičia
rename-nothing-to-undo = nėra grupinio pervadinimo, kurį būtų galima atšaukti
rename-done =
    { $count ->
        [one] pervadintas { $count } failas · atšaukti galima iš meniu „Failai“
        [few] pervadinti { $count } failai · atšaukti galima iš meniu „Failai“
       *[other] pervadinta { $count } failų · atšaukti galima iš meniu „Failai“
    }
rename-undone =
    { $count ->
        [one] grąžintas { $count } vardas
        [few] grąžinti { $count } vardai
       *[other] grąžinta { $count } vardų
    }
pack-title = Supakuoti į archyvą
pack-name = Archyvas
pack-format = Formatas
pack-level = Glaudinimas
pack-solid = Vientisi blokai (7z: mažesnis, bet vieną failą skaityti lėčiau)
pack-delete-after = Suarchyvavus failus ištrinti
pack-level-normal = įprastas
pack-level-fast = greitas
pack-level-best = geriausias
pack-scope = pažymėta failų: { $count }
pack-needs-name = archyvui reikia vardo
pack-format-read-only = { $format } čia tik skaitomas, ne rašomas: pasirinkite kitą formatą arba kitą vardą
pack-single-one-file = { $format } talpina vieną failą: pasirinkite vieną arba katalogui ar keliems failams rinkitės { $tar }
unpack-title = Išarchyvuoti iš archyvo
unpack-to = Išarchyvuoti į
unpack-scope = iš { $name }
unpack-not-an-archive = tai ne archyvas, kurį mokame skaityti

# ── the special panel kinds (SPEC 02 §7) ────────────────────────────────────────────────
panel-quick-view = Vaizdas
panel-info = Informacija
panel-tree = Medis
compare-done = skiriasi: { $left } kairėje, { $right } dešinėje
cmp-subdirs = Eiti į pakatalogius
cmp-depth = Kaip giliai eiti (0 = iki pat galo)
cmp-trees-title = Kas skiriasi
cmp-trees-working = lyginami medžiai: failų { $seen }, perskaityta { $read }
cmp-trees-done = skiriasi { $differ }, tik kairėje { $left }, tik dešinėje { $right }, iš { $pairs } porų
cmp-trees-same = abu medžiai vienodi (porų { $pairs }, patikrinta { $same })
menu-duplicates = Rasti dublikatus
dup-title = Failų dublikatai
dup-working = ieškoma dublikatų: failų { $seen }, perskaityta { $hashed }
dup-done = kopijų grupių { $groups }, failų { $files }, iššvaistyta { $wasted }
dup-none = niekas čia nėra ko nors kito kopija (failų: { $seen })
dup-stopped = sustabdyta po { $seen } failų
dup-partial = sustabdyta anksčiau laiko
cmp-title = Palyginti
cmp-case = Varduose skirti didžiąsias ir mažąsias raides
cmp-size = Kitoks dydis yra skirtumas
cmp-time = Kitoks laikas yra skirtumas
cmp-seconds = Laikai, besiskiriantys tiek sekundžių, laikomi tuo pačiu momentu
cmp-contents = Skaityti baitus
cmp-when-time = Skaityti juos tik ten, kur skiriasi laikai
cmp-fragment = Skaityti tik tiek pirmųjų KiB (0 = viską)
cmp-ignore-eol = Vien eilučių pabaigos nėra skirtumas
cmp-selected = Tik tai, kas pažymėta
cmp-stop = Sustoti ties pirmu skirtumu
cmp-newer = Pažymėti tik naujesnę kopiją
menu-compare-setup = Palyginti su pasirinkti&mis
compare-contents-eol-ignored = { $count } skiriasi tik eilučių pabaigomis, nepaisoma
compare-contents-trusted = { $count } laikomi vienodais neskaičius
compare-contents-working = Lyginamas turinys: { $done } iš { $total }
compare-contents-done = skiriasi { $differ } iš { $pairs } porų
compare-contents-same = visos { $pairs } poros vienodos
compare-contents-none = nė vienas failas neturi to paties vardo abiejose pusėse
compare-contents-local = lyginant pagal turinį abu langai turi būti vietinėje failų sistemoje
compare-contents-eol = { $count } tik eilučių pabaigomis
compare-contents-only = { $count } tik vienoje pusėje
compare-contents-unread = { $count } nepavyko perskaityti
compare-contents-stopped = lyginimas sustabdytas
quick-view-directory = { $name } yra katalogas — F3 jį išmatuoja
quick-view-not-here = baitų čia nėra: F3 pirmiausia juos atsiunčia
info-path = Kelias
info-kind = Rūšis
info-kind-local = katalogas šiame kompiuteryje
info-kind-remote = serveryje { $host }
info-kind-archive = archyve { $name }
info-kind-list = failų sąrašas
info-device = Įrenginys
info-folder = Katalogas
info-device-value = { $total }, laisva { $free }, klasteriai { $cluster }
info-slack = Nuostoliai klasteriuose
info-filesystem = Failų sistema
info-mount = Prijungta
info-git = Saugykla
info-git-clean = sutampa su indeksu
info-git-differing = { $count } iš { $tracked } skiriasi nuo indekso
info-git-uncounted = nesuskaičiuota
info-git-merge = vyksta merge
info-git-rebase = vyksta rebase
info-git-cherry-pick = vyksta cherry-pick
info-git-revert = vyksta revert
info-git-bisect = vyksta bisect
info-memory = Atmintis
info-memory-value = laisva { $free } iš { $total }
info-memory-total = { $total }
info-contents = Turinys
info-contents-value = failų: { $files }, katalogų: { $dirs }, { $bytes }
info-selected = Pažymėta
info-selected-value = failų: { $count }, { $bytes }
info-cursor = Žymeklis
info-size = Dydis
info-modified = Modifikuotas
info-description = Aprašymas
key-mark = Žymėti
key-asm = Asm
key-block = Blokas
key-screens = Ekran
key-site-edit = RedSes
key-site-new = NaujSes
key-site-forget = TrinSes
key-ai-edit = Redag
key-ai-new = Nauja
key-ai-remove = Šalinti
key-ai-forget = Pamiršt
key-ai-threads = Istor
hex-needs-a-file = šis buferis neturi failo, iš kurio būtų galima skaityti baitus
hex-save-first = pirmiausia išsaugokite: du redaktoriai negali laikyti vienas kito neišsaugotų pakeitimų

# ── the hex editor's block operations (SPEC 05 §2.6) ────────────────────────────────────
hex-block-title = Blokas
hex-op-write = Įrašyti { $size } į failą
hex-op-read = Įskaityti failą ties { $at }
hex-op-insert = Įterpti nulinius baitus ties { $at }
hex-op-copy = Kopijuoti baitus iš kitur į { $at }
hex-op-move = Perkelti baitus iš kitur į { $at }
hex-op-fill = Užpildyti { $size } baitu
hex-op-xor = Pritaikyti XOR { $size } su raktu
hex-op-delete = Ištrinti { $size } — { $was } tampa { $becomes }
hex-write-title = Įrašyti bloką į
hex-read-title = Kurį failą įskaityti
hex-fill-title = Kuriuo baitu užpildyti (hex)
hex-xor-title = XOR su kuriuo raktu (hex)
hex-insert-title = Kiek nulinių baitų
hex-copy-title = Iš kur kopijuoti — poslinkis ir ilgis (hex)
hex-move-title = Iš kur perkelti — poslinkis ir ilgis (hex)
hex-not-hex = tai ne šešioliktainių skaitmenų seka
hex-not-a-count = tai ne baitų skaičius
hex-not-a-range = tai ne šešioliktainis poslinkis ir ilgis
hex-range-outside = šis intervalas yra už failo ribų ({ $size })
hex-move-into-itself = bloko negalima perkelti į jį patį
hex-filled = užpildyta: { $size }
hex-xored = { $size } pritaikytas XOR — tai nėra šifravimas
hex-written = { $size } įrašyta į { $name }
hex-read-done = { $size } įskaityta iš { $name }
hex-inserted = įterpta nulių: { $size }
hex-deleted = ištrinta: { $size }
hex-device = { $name }: { $size } dydžio įrenginys, atvertas tik skaitymui
asm-title = Asembliuoti ties { $at } ({ $arch })
asm-no-arch = { $arch } nėra architektūra, kuriai mokame asembliuoti
asm-written = { $text } — { $bytes }
asm-written-longer = { $text } — { $bytes } ({ $grew } baitais ilgiau)
screens-title = Ekranai
screens-panels = Langai
quit-title = Išeiti iš FinalCommander
quit-body = Ar norite išeiti?
screens-quit-title = Neišsaugoti pakeitimai
screens-quit-body = Šiuose ekranuose yra pakeitimų, kurių niekas neįrašė:
macro-recording = Įrašoma makrokomanda · Ctrl+. ją sustabdo
macro-empty = nieko neįrašyta
macro-dropped = įrašas išmestas
macro-key-title = Kuris klavišas
macro-key-body = Įrašyta žingsnių: { $steps }. Paspauskite klavišą, į kurį ši makrokomanda atsilieps, arba Esc, kad ją išmestumėte.
macro-describe = Kam skirta klavišo { $key } makrokomanda
macro-saved = klavišo { $key } makrokomanda išsaugota, žingsnių: { $steps }
macro-none = makrokomandų dar neįrašyta · Ctrl+. įrašo vieną
macro-list-title = Makrokomandos
macro-play-title = Kaip ją vykdyti
macro-play-once = Vieną kartą, nuo ten, kur žymeklis
macro-play-each = Po kartą kiekvienam pažymėtam failui
macro-sure-title = Vykdyti šią makrokomandą
macro-sure-body = Klavišo { $key } makrokomanda kopijuoja, perkelia, trina arba perrašo. Vykdyti ją?
macro-too-deep = makrokomanda negali vykdyti pati savęs be galo
plugin-list-title = Priedų komandos
plugin-none-title = Priedų neįdiegta
plugin-none-open = Atverti priedų katalogą ({ $dir })
plugin-none-help = Kaip veikia priedai ir kur jų rasti
plugin-help-heading = Priedai
plugin-why-off = išjungtas: Opcijos → Priedų konfigūracija
plugin-why-panel = tik languose
plugin-why-viewer = tik peržiūroje
plugin-why-editor = tik redaktoriuje
plugin-why-local = reikia katalogo šio kompiuterio diskuose
plugin-why-files = reikia failo po žymekliu arba pažymėto
plugin-why-masks = tik šiems: { $masks }
plugin-why-bad-mask = jo maskuotė klaidinga: { $error }
plugin-why-here = šiame kataloge reikia { $names }
plugin-why-inside = čia arba aukštesniame kataloge reikia { $names }
plugin-cannot = { $label }: { $why }
plugin-running = { $label }…
plugin-on-selected = pažymėtiems failams: { $count }
plugin-failed = { $id }: { $why }
plugin-unknown-command = priedas paprašė `{ $id }`, o tai nėra komanda
hex-copied = { $size } nukopijuota iš { $from }
hex-moved = { $size } perkelta iš { $from }
menu-calculator = Skaičiuotuvas
menu-terminal = Terminalas
calc-title = Skaičiuotuvas
calc-decimal = dešimtainis
calc-hex = šešioliktainis
calc-octal = aštuntainis
calc-binary = dvejetainis
calc-characters = simboliai

# The archive commands (Shift+F3) and the two parts of the order that are not the key.
arc-commands-title = Archyvo komandos
arc-password = { $name } slaptažodis
arc-op-test = Tikrinti { $name }
arc-op-extract-here = Išarchyvuoti { $name } čia
arc-op-extract-to = Išarchyvuoti { $name } į…
arc-op-delete = Ištrinti { $name }
arc-op-add = Pridėti failus į archyvą…
job-test = Tikrinu archyvą
msg-dirs-first-on = pirmiau katalogai
msg-dirs-first-off = katalogai kartu su failais
msg-selected-first-on = pirmiau pažymėti failai
msg-selected-first-off = pažymėti failai iš eilės

# The change-drive dialog (Alt+F1, Alt+F2): the type column, and its footer.
drive-fixed = Fiksuotas
drive-removable = Keičiamas
drive-optical = Optinis
drive-network = Tinklas
drive-memory = Atmintis
drive-home = Namų katalogas
drive-connection = Prisijungimas
drive-objects = Objektų saugykla
drive-netbox = NetBox: išsaugotos sesijos
drive-processes = Procesų sąrašas
drive-temp = Laikinas langas
temp-added = pridėta į laikiną langą: { $count }
proc-none = veikiančių procesų sąrašo čia perskaityti nepavyksta
drive-programs = Įdiegtos programos
drive-services = Tarnybos
svc-none = šis kompiuteris neturi tarnybų tvarkytuvės, kurios galėtume paklausti
temp-not-a-file = eilutės čia nėra failai: nėra ko trinti
svc-card = Tarnyba
svc-card-id = Vardas
svc-card-name = Rodomas vardas
svc-card-state = Būsena
svc-card-description = Aprašymas
svc-card-pid = Procesas
svc-state-running = veikia
svc-state-stopped = sustabdyta
svc-state-failed = sutriko
svc-state-other = kita
key-service-start = Paleist
key-service-restart = Perkrau
key-service-stop = Stabdyt
svc-ask-stop = Sustabdyti { $name }?
svc-ask-restart = Paleisti { $name } iš naujo?
btn-service-stop = Sustabdyti
btn-service-restart = Paleisti iš naujo
svc-working = { $name }: klausiama sistemos…
svc-started = { $name } paleista
svc-stopped = { $name } sustabdyta
svc-restarted = { $name } paleista iš naujo
svc-needs-root = { $name }: tam reikia root teisių — programa jų už jus neprašys
svc-failed = { $name }: { $error }
prog-none = niekas čia nenurodo, kas įdiegta
prog-card = Įdiegta programa
prog-no-uninstall = niekas nenurodo, kaip pašalinti { $name }
prog-uninstall-ready = komanda, pašalinanti { $name }, yra komandinėje eilutėje, nepaleista
prog-card-name = Pavadinimas
prog-card-version = Versija
prog-card-publisher = Leidėjas
prog-card-size = Dydis diske
prog-card-installed = Įdiegta
prog-card-location = Katalogas
prog-card-uninstall = Pašalinama komanda
drive-sockets = Lizdų sąrašas
sock-none = čia niekas nesiklauso arba lentelės nepavyksta perskaityti
sock-no-process = niekas nepasako, kuris procesas laiko šį lizdą
proc-card = Procesas
proc-gone = { $name } nebeveikia
proc-card-name = Vardas
proc-card-pid = Proceso id
proc-card-parent = Paleido
proc-card-user = Vartotojas
proc-card-state = Būsena
proc-card-threads = Gijos
proc-card-memory = Atmintis
proc-card-mapped = Atvaizduota
proc-card-bits = Bitų skaičius
proc-card-started = Paleistas
proc-card-exe = Vykdomasis failas
proc-card-cwd = Katalogas
proc-card-command = Komandinė eilutė
proc-card-group = Grupė
proc-card-sid = SID
proc-card-priority = Prioritetas
proc-card-session = Seansas
proc-card-uptime = Veikia jau
proc-card-cpu-user = Procesoriaus laikas, savas
proc-card-cpu-kernel = Procesoriaus laikas, branduolyje
proc-card-handles = Atviros rodyklės
proc-card-faults = Puslapių klaidos
proc-card-private = Privati atmintis
proc-card-swap = Iškelta į mainų sritį
proc-card-io-read = Perskaityta
proc-card-io-write = Įrašyta
proc-card-memory-peak = Darbinio rinkinio maksimumas
proc-card-private-resident = Darbinis rinkinys, privatus
proc-card-swap-peak = Puslapių failo maksimumas
proc-card-virtual = Virtualūs baitai
proc-card-virtual-peak = Virtualių baitų maksimumas
proc-card-paged-pool = Puslapiuojamas telkinys
proc-card-nonpaged-pool = Nepuslapiuojamas telkinys
proc-card-io-other = Kitas įv./išv.
proc-card-io-read-ops = Skaitymo operacijos
proc-card-io-write-ops = Rašymo operacijos
proc-card-io-other-ops = Kitos operacijos
proc-kill-title = Baigti procesą
proc-kill-ask = Baigti { $name } [{ $pid }]? Tai, ko jis neišsaugojo, bus prarasta.
proc-killing = Baigiamas { $name }…
proc-killed = { $name } baigtas
proc-kill-failed = { $name } nepavyko baigti: { $why }
proc-card-env = Aplinka
proc-card-modules = Įkelti moduliai
proc-card-version = Versija
proc-card-description = Aprašymas
drives-hint = ↑↓ rinktis · 0-9 langai · Ctrl+1..9 stulpeliai · F9 visi · Ctrl+R atnaujinti
drives-title = Keisti diską: ką rodyti
drives-col-type = tipą
drives-col-device = įrenginį
drives-col-label = disko vardą
drives-col-fs = failų sistemą
drives-col-size = dydžius
drives-col-removable = keičiamus diskus
drives-col-panels = langus, kurie nėra diskai
drives-col-optical = optinius diskus
drives-col-network = tinklo išteklius
drives-shown = rodoma: { $what }
drives-hidden = slepiama: { $what }
drives-reread = diskai perskaityti iš naujo
bookmarks-hint = Enter eiti · Ins įsiminti · Del šalinti · F4 pavadinti · Esc uždaryti

# The search dialog (F7), which asks the whole question in one place.
search-title = Paieška
search-for = Ko ieškoti
search-mode = Kaip
search-mode-text = tekstas
search-mode-hex = hex
search-mode-regex = reguliarusis reiškinys
search-case = Didžiosios skiriasi
search-words = Tik visi žodžiai
btn-find-next = Rasti kitą
btn-find-prev = Rasti ankstesnį
# The help: the chapters' own names, and the note on a chapter nobody has translated yet
help-chapter-about = Kas tai
help-chapter-panels = Langai
help-chapter-files = Failai: kopijavimas, perkėlimas, paieška, archyvai
help-chapter-viewer = Peržiūra
help-chapter-editor = Redaktorius, hex skydelis ir asembleris
help-chapter-network = Serveriai
help-chapter-config = Konfigūracija, klavišai ir spalvos
help-chapter-keys = Klavišai, kaip jie priskirti dabar
help-chapter-commands = Visos komandos ir jų klavišai
help-chapter-options = Visi nustatymai ir jų reikšmės
help-here-panels = Langai
help-here-viewer = Peržiūra
help-here-editor = Redaktorius
help-here-hex = Hex redaktorius
help-here-dialog = Šis dialogas
help-in-english = Šis skyrius į jūsų kalbą dar neišverstas, todėl toliau pateikiamas angliškai. Jame minimi klavišai ir nustatymai visose kalbose tie patys.
help-about-byline = FinalCommander { $version } — autorius Matteo Baccan
key-ok = Gerai
key-cancel = Atšaukt
key-dialog-move = Stumti
key-tree = Medis
key-config = Konfig
browse-local-only = failas pasirenkamas iš šio kompiuterio; kelias serveryje įvedamas, o Tab jį užbaigia
key-browse = Naršyti
table-prompt-filter = Eilutės, kuriose yra
table-sorted = surūšiuota pagal { $column }
table-sorted-back = surūšiuota pagal { $column }, atvirkščiai
table-unsorted = grįžta prie failo tvarkos
table-filtered = liko eilučių: { $rows }
table-unfiltered = vėl visos eilutės
table-cut-sorted = iš pirmųjų { $rows } eilučių — tiek tik ir buvo perskaityta
table-too-many = per daug įrašų, kad čia būtų galima rūšiuoti ar filtruoti
table-copied =
    { $rows ->
        [one] { $rows } eilutė iškarpinėje
        [few] { $rows } eilutės iškarpinėje
       *[other] { $rows } eilučių iškarpinėje
    }
table-headed = pirma eilutė yra antraštė
table-not-headed = pirma eilutė yra duomenys: stulpeliai vadinami pagal numerį
table-named-fields = šis formatas pats įvardija savo laukus: nėra antraštės, kurią reikėtų spėti

# Splitting and combining (SPEC 03 §9)
split-title = Padalinti failą
split-target = Į
split-size = Kiekviena dalis
split-size-hint = 1440K, 700M, 4G — paskutinė dalis yra tai, kas lieka
split-count = Arba į
split-count-hint = dalis — čia įrašytas skaičius viršesnis už dydį
split-bad-count = { $count } nėra dalių skaičius: reikia dviejų ar daugiau
split-local-file = Dalinimas ir jungimas veikia su failu šiame kompiuteryje
split-bad-size = „{ $size }“ nėra dydis: rašykite 700M arba 4G
combine-not-a-piece = Tai ne dalis: padalinto failo dalys baigiasi .001, .002, … arba turi šalia .crc
pieces-working = Dalinama arba jungiama: { $done } iš { $total }
pieces-cancelled = Sustabdyta, tai, kas buvo įrašyta, pašalinta
split-done = įrašyta dalių: { $parts }, šalia jų — { $list }
combine-done-checked = { $name } sujungtas iš { $parts } dalių, { $bytes }, ir sutampa su savo .crc
combine-done-unchecked = { $name } sujungtas iš { $parts } dalių, { $bytes } — nebuvo .crc, pagal kurį būtų galima patikrinti
pieces-failed = Neatlikta: { $error }

# The branch view (SPEC 02 §3d)
branch-reading = Skaitomi visi failai kataloge { $path }…
branch-title = Šaka: { $path }
branch-done = failų: { $count }, iš visų žemiau esančių katalogų · Ctrl+Alt+B grąžina
branch-truncated = failų: { $count }, ir čia peržiūra sustojo: šaka didesnė, nei sąrašas gali būti naudingas
branch-not-here = Šakos vaizdas rodomas katalogui, o šis langas yra sąrašas

## The release check (SPEC 11 §5).
menu-check-update = Tikrinti atna&ujinimus
opt-update-check = Kartą per dieną tikrinti, ar yra nauja laida
btn-open-page = Atverti puslapį
update-checking = Klausiama GitHub apie naujausią laidą…
update-title = Atnaujinimai
update-available = Išleistas FinalCommander { $version } (čia — { $current }).
update-available-status = Išleistas FinalCommander { $version }: Opcijos → Tikrinti atnaujinimus
update-current = Tai naujausia laida, { $version }.
update-failed = Nepavyko sužinoti naujausios laidos: { $why }

## Duplicate files (SPEC 06 §13).
menu-dupes = Rasti failų dublikatus
dupes-title = Failų dublikatai
dupes-recursive = Ieškoti pakatalogiuose
dupes-min-size = Mažiausias failas
dupes-min-size-hint = Dydis, kaip jį rašo filtrai: 1, 100K, 5M. Tušti failai niekada nelyginami.
dupes-select = Pažymėti visas kopijas, išskyrus seniausią
btn-search = Ieškoti
dupes-not-here = Dublikatų paieška pradedama nuo katalogo arba nuo to, kas pažymėta.
dupes-bad-size = Ne dydis: { $size }
dupes-looking = Ieškoma failų dublikatų…
dupes-progress = Dublikatai: etapas { $pass } iš 3, { $done }/{ $total }
dupes-none = Tarp { $count } failų dublikatų nėra.
dupes-panel = Dublikatai: grupių { $groups }
dupes-done = dublikatų grupių: { $groups }, kopijose { $size }.
dupes-truncated = Sustabdyta anksčiau laiko: kol kas grupių { $groups }, kopijose { $size }.

## SQLite queries (SPEC 04 §4b).
sql-empty = Parašykite užklausą po sql:, pavyzdžiui, sql: SELECT * FROM t
sql-no-database = sql: reikia SQLite duomenų bazės: įeikite į ją arba pastatykite ant jos žymeklį.
sql-running = Vykdoma užklausa…
sql-failed = Užklausa nepavyko: { $why }

## Asking a language model (SPEC 06 §14).
ai-none = Nenustatyta jokia DI paslauga: pridėkite [[ai]] bloką į fcmd.toml (name, url, model, api, token_env).
ai-title = Klausti DI
ai-no-thread = nėra atsakymo, kurį būtų galima tęsti: pirmiausia paklauskite su `ai:` arba pastatykite žymeklį ant atsakymo atsakymų kataloge
ai-nothing-to-explain = čia nėra ko aiškinti: atverkite failą peržiūroje ar redaktoriuje arba klauskite iškart po nepavykusios komandos
ai-explain-question = Kas tai, iš { $name }? Paaiškink trumpai.
ai-tokens = žetonų: { $in } įeina, { $out } išeina
ai-none-panel = DI paslaugos dar nėra — F7 ją prideda
ai-service-removed = { $name } pašalinta
ai-probing = Jungiamasi prie { $name } adresu { $host }…
ai-probe-ok = { $host } atsakė HTTP { $status } per { $ms } ms, modelių: { $models } — įveskite klausimą ir spauskite Enter
ai-probe-no-model = (modelio { $model } tarp jų nėra)
ai-row-context = kontekstas
ai-row-conversation = pokalbis
ai-session-title = DI: { $name } · { $model } — failų kontekste: { $files }
ai-context-added = { $count ->
        [one] { $count } failas
        [few] { $count } failai
       *[other] { $count } failų
    } kontekste
ai-context-nothing = Nėra ko pridėti: į kontekstą patenka tik šio disko failai
ai-command-ready = komanda yra eilutėje — perskaitykite ją prieš spausdami Enter
ai-commit-question = Parašyk commit pranešimą paruoštiems pakeitimams.
ai-staged-diff = paruoštas diff
ai-send-diff = Paruoštas diff siunčiamas į { $host }
ai-send-files = Failai ({ $count }, { $size }) bus išsiųsti į { $host } kartu su klausimu.
ai-asking = Klausiama { $name }…
ai-busy = { $name } galvoja… { $secs } s · Esc sustabdo
ai-conversation-new = Naujas pokalbis: jį pradės kitas klausimas.
ai-live-empty = Įveskite klausimą komandinėje eilutėje: jis keliauja į { $name }, o pokalbis čia rodomas jam vykstant.
ai-failed = DI neatsakė: { $why }
ai-stopped = sustabdyta: klausimas, skirtas { $name }, atmestas, niekas neįrašyta
ai-context-title = Failai kontekste
ai-context-all = Visi
ai-context-dropped = { $count ->
        [one] { $count } failas išimtas iš konteksto
        [few] { $count } failai išimti iš konteksto
       *[other] { $count } failų išimta iš konteksto
    }
ai-context-hint = Enter arba Del: išimti iš konteksto — failas lieka diske · F3: kas siunčiama · Esc
ai-threads-title = Pokalbiai
ai-threads-hint = Enter: tęsti šį pokalbį · Esc
ai-thread-resumed = tęsiamas { $name }
ai-next-estimate = kitas ≈ { $tokens }
ai-next-over = kitas ≈ { $tokens }, daugiau, nei telpa į modelį
ai-send-estimate = Iš viso apie { $tokens } žetonų, su ligšioliniu pokalbiu.
ai-send-estimate-of = Iš viso apie { $tokens } žetonų, su ligšioliniu pokalbiu, iš { $max }, kurie telpa į modelį.
ai-send-over = Iš viso apie { $tokens } žetonų: daugiau nei { $max }, kurie telpa į modelį, todėl pradžia gali būti nukirpta. F8 išima failą.
ai-send-question = Klausimas siunčiamas į { $host }.
btn-send = Siųsti

## Git (SPEC 06 §15).
menu-git = Git
git-title = Git
git-status = Kas pasikeitė
git-log = Žurnalas
git-blame = Blame { $name }
git-branches = Šakos
git-local-only = git dirba su katalogu šiame diske.
git-running = Klausiama git…
git-failed = git: { $why }
git-clean = Niekas nepasikeitė.
git-status-title = Pakeista: paruošta { $staged }, neparuošta { $unstaged }
git-side-conflict = konfliktas
git-side-staged = paruošta
git-side-changed = neparuošta
git-side-untracked = nesekama
git-kind-modified = pakeistas
git-kind-added = naujas
git-kind-deleted = ištrintas
git-kind-renamed = pervadintas
git-kind-copied = nukopijuotas
git-kind-type = pakeistas tipas
git-kind-conflict = spręstinas
git-as-tree = Rodyti pakeitimus kaip medį
git-as-list = Rodyti pakeitimus kaip sąrašą
git-version-head = paskutinis commit
git-version-index = paruošta
git-version-disk = diske
git-discard-title = Atmesti pakeitimus
git-discard-body = Atmesti { $count ->
        [one] { $count } failo
       *[other] { $count } failų
    } pakeitimus?
git-discard-worktree = atgal į indekso būseną:
git-discard-both = atgal į paskutinį commit:
git-discard-added = iš indekso, paliekama diske:
git-discard-trash = į šiukšlinę:
git-discard-warning = To, kas grįžta į indekso ar paskutinio commit būseną, atšaukti nebegalima.
git-hint = Enter: rinktis · Esc: uždaryti

## The editor's character list and word completion.
charmap-title = Simboliai
charmap-hint = Rašykite, kad susiaurintumėte (simbolis arba jo kodas, U+00E9) · Enter: įterpti · Esc: atgal
editor-no-completion = Joks kitas žodis faile taip neprasideda.

## A stored session's address, copied (NetBox's Generate URL).
net-url-copied = Nukopijuota { $url }

## The system log (SPEC 06 §16).
menu-syslog = Sistemos žurnalas
syslog-reading = Skaitomas sistemos žurnalas…

## Deleting lines by pattern (FAR #10).
editor-prompt-delete-matching = Pašalinti eilutes, atitinkančias (! pradžioje: palikti tik jas)
editor-lines-deleted = Pašalinta eilučių: { $count }

## The copy dialog's size policy (FAR #493).
copy-conflict-size = perrašyti, jei skiriasi dydis

# Copy dialog: what an error does (FAR's issue #597)
copy-on-error = Jei kurio nors nepavyksta nukopijuoti
copy-error-ask = Klausti
copy-error-skip = Praleisti ir tęsti
copy-error-stop = Sustabdyti kopijavimą

# Attributes dialog: the times copied from one file to another (FAR's issue #241)
attr-times-copied = Laikai nukopijuoti: Ctrl+Shift+V įdeda juos į kito failo dialogą
attr-times-none = Dar nieko nenukopijuota: pirmiausia Ctrl+Shift+C failo dialoge
attr-times-pasted = Laikai įklijuoti: „Gerai“ juos nustato

# Editor: replacements kept by name and recent ones (FAR's issue #282)
replace-tpl-title = Pakeitimai
replace-tpl-hint = Enter vykdo · Ins išsaugo neseną vardu · Del pamiršta išsaugotą · Esc
replace-tpl-name = Išsaugoti šį pakeitimą vardu
replace-tpl-kept = Išsaugota vardu {$name}
replace-tpl-empty = Pakeitimų dar nėra: pirmiausia Ctrl+F7

# Panel settings: a view and a sort per folder (FAR's issue #389)
opt-remember-view = Kiekvienas katalogas išlaiko savo vaizdą ir rūšiavimą

# Session editor: a cap on a copy's speed to or from the site
site-max-kbps = Greičio riba, KB/s (tuščia: nėra)

# Session editor: IMAP, read only
site-protocol-imap = IMAP (paštas, tik skaitymas)
site-protocol-imaps = IMAPS (paštas per TLS, tik skaitymas)

# The AI services as a list, like the sessions
ai-services-title = DI paslaugos
ai-services-hint = Enter: jos klausia ai: · Ins nauja · F4 keisti · Del šalinti · Esc
ai-service-title = DI paslauga
ai-field-name = Vardas
ai-field-api = Protokolas
ai-field-url = Adresas
ai-field-model = Modelis
ai-field-token-env = Raktas kintamajame
ai-field-token-note = Tuščia: rakto paprašoma kartą per seansą ir jis niekur neužrašomas.
ai-field-system = Kas jai pasakoma pirmiausia
ai-service-incomplete = Paslaugai reikia vardo ir adreso
ai-service-default = ai: dabar klausia {$name}
ai-key-prompt = {$name} raktas (laikomas, kol išeisite)
ai-api-openai = OpenAI ir suderinamos
ai-api-anthropic = Anthropic
ai-api-ollama = Ollama (šiame kompiuteryje)
menu-ai = DI paslaugos

# The secrets file
vault-password = fcmd paslapčių failo slaptažodis
vault-new = Laikyti šį raktą šifruotame fcmd paslapčių faile: pasirinkite jo slaptažodį (tuščia: tik šiam seansui)
vault-refused = Paslapčių failas neatsivėrė: {$why}

# The programs from elsewhere
drive-tools = Išorinės programos
drive-ai = DI
drive-git = Git
tools-found = rasta
tools-missing = neįdiegta
tools-summary = rasta išorinių programų: { $found } iš { $total }; F3 pasako, kam kuri skirta, F4 įdiegia trūkstamą
tools-card = Išorinė programa
tools-card-program = Programa
tools-card-where = Kur
tools-card-version = Versija
tools-card-for = Kam naudojama
tools-card-install = Kaip įdiegti
tools-card-why = Kodėl fcmd ją paleidžia, o ne daro tai pati: F1, „Programos iš kitur“.
tools-install-any = įdiekite paketą, kuriame yra { $name }, ir įsitikinkite, kad ji yra PATH
tools-already = { $name } čia jau yra
tools-for-archives = archyvai: { $what }
tools-for-decompile = Shift+F6 peržiūroje failams { $what }
tools-for-git = Git langas (Ctrl+Alt+G) ir šaka raginime
tools-for-video = vaizdo įrašo kadrai peržiūroje ir greitajame vaizde (F3, Ctrl+Q)
tools-for-open = nuorodos atvėrimas naršyklėje
tools-for-syslog = Komandos → Sistemos žurnalas
tools-for-services = tarnybų langas
tools-card-site = Svetainė
tools-no-package = Nežinoma, kad kuri nors šio kompiuterio paketų tvarkytuvė turėtų { $name }. Atverti atsisiuntimo puslapį?
tools-install-ask = Įdiegti { $name } terminale paleidžiant:
tools-install-how = Terminale sudo paprašys jūsų slaptažodžio. Ctrl+Alt+E grąžina į langus, o Ctrl+R šiame lange patikrina iš naujo.
tools-install-else = Jei nepavyks: { $url }
tools-install-failed = fcmd: { $name } neįdiegta. Atsisiųskite iš { $url }
tools-installing = { $name } diegiama terminale: Ctrl+Alt+E grąžina į langus, Ctrl+R ten patikrina iš naujo
btn-install = Įdiegti

# Plugins configuration
plugins-config-title = Priedai
plugins-config-hint = Enter: įjungti arba išjungti · F3 kas tai ir ką jis duoda · Esc
plugins-broken = neįsikėlė — F3 pasako kodėl
plugins-cannot-toggle = neįsikėlusio priedo įjungti negalima: F3 pasako, kas jam negerai
plugins-on = { $id } įjungtas: jo komandos dabar yra Ctrl+Shift+F11
plugins-on-restart = { $id } įjungtas: jo komandos dabar yra Ctrl+Shift+F11, visa kita atsiras nuo kito paleidimo
plugins-off = { $id } išjungtas: jo komandų Ctrl+Shift+F11 nebėra
plugins-off-restart = { $id } išjungtas: jo komandų Ctrl+Shift+F11 nebėra, visa kita dings per kitą paleidimą
plugins-state-on = įjungtas
plugins-state-off = išjungtas
plugins-card-id = Id
plugins-card-name = Vardas
plugins-card-version = Versija
plugins-card-author = Autorius
plugins-card-about = Apie
plugins-card-dir = Katalogas
plugins-card-state = Būsena
plugins-card-brings = Ką duoda
plugins-card-commands = Komandos (Ctrl+Shift+F11):
plugins-card-notes = Pranešimai įkeliant:
plugins-none = priedų nerasta · jie gyvena { $dir }, kiekvienas savo kataloge su plugin.toml

# The git panel
git-file-log = { $name } žurnalas
git-commit = Įrašyti (commit) tai, kas paruošta…
git-pull = Pull
git-push = Push
git-pushing = Vykdomas push…
git-pulling = Vykdomas pull…
git-commit-prompt = Commit pranešimas
git-commit-empty = Commit reikia pranešimo.
git-already-on = Jūs jau esate šakoje { $branch }.
git-switch-ask = Persijungti į { $branch }? Failai, kuriais šakos skiriasi, diske pasikeis.
git-done = git: { $said }
git-branches-title = Šakos: { $count }
git-log-title = Žurnalas: { $count }
git-log-of-title = { $of } žurnalas: { $count }
btn-switch = Persijungti
key-git = Git
key-git-diff = Diff
key-git-stage = Paruošt
key-git-unstage = Išimti
key-git-discard = Atmesti
key-git-commit = Commit
key-git-show = Rodyti
key-git-log = Žurnal
key-git-changes = Pakeit
key-git-pull = Pull
key-git-push = Push
key-git-branches = Šakos

undo-title = Atšaukti
btn-undo = Atšaukti
menu-undo = Atšaukti paskutinę operaciją
menu-strip-meta = Pašalinti paveikslėlių metaduomenis…
undo-nothing = nėra ko atšaukti: nuo programos paleidimo nebuvo kopijavimo, perkėlimo, trynimo į šiukšlinę ar naujo katalogo
undo-copy =
    { $count ->
        [one] Nusiųsti kopijavimo sukurtą { $count } failą į šiukšlinę:
        [few] Nusiųsti kopijavimo sukurtus { $count } failus į šiukšlinę:
       *[other] Nusiųsti kopijavimo sukurtų { $count } failų į šiukšlinę:
    }
undo-move =
    { $count ->
        [one] Perkelti { $count } failą atgal, kur jis buvo:
        [few] Perkelti { $count } failus atgal, kur jie buvo:
       *[other] Perkelti { $count } failų atgal, kur jie buvo:
    }
undo-trash =
    { $count ->
        [one] Atkurti { $count } failą iš šiukšlinės:
        [few] Atkurti { $count } failus iš šiukšlinės:
       *[other] Atkurti { $count } failų iš šiukšlinės:
    }
undo-mkdir = Pašalinti naują katalogą, jei jis vis dar tuščias:
strip-title = { $count ->
        [one] { $count } paveikslėlio metaduomenys
       *[other] { $count } paveikslėlių metaduomenys
    }
strip-all = Viską — tekstą, EXIF, XMP, IPTC, komentarus, C2PA
strip-ai = Tik tai, ką įrašė vaizdų modelis — užklausą, darbo eigą, nustatymus
strip-camera = Viską, išskyrus fotoaparato EXIF ir autorių teises — be GPS
strip-hint = Enter: pašalinti; paveikslėlis neperkoduojamas, Ctrl+Alt+Z grąžina · Esc
strip-nothing = šiame diske nepažymėtas ir po žymekliu nėra jokio PNG, WebP ar JPEG
strip-done = { $count ->
        [one] { $count } paveikslėlis
        [few] { $count } paveikslėliai
       *[other] { $count } paveikslėlių
    } be metaduomenų (pašalinta blokų: { $blocks }), neturėjo ko šalinti: { $clean } · Ctrl+Alt+Z juos grąžina
undo-restore = { $count ->
        [one] Grąžinti { $count } paveikslėlį su metaduomenimis:
        [few] Grąžinti { $count } paveikslėlius su metaduomenimis:
       *[other] Grąžinti { $count } paveikslėlių su metaduomenimis:
    }
undo-done =
    { $count ->
        [one] { $count } vardas grąžintas, kur buvo
        [few] { $count } vardai grąžinti, kur buvo
       *[other] { $count } vardų grąžinta, kur buvo
    }
slow-drives = diskų sąrašas užtruko { $secs } s
slow-drives-late = diskų sąrašas laukė { $secs } s: { $drives } neatsakė laiku ir rodomi be detalių — gal tinklo diskas, kurio serverio nėra?
slow-listing = { $what } skaitymas užtruko { $secs } s
# editor search and undo (0.41)
btn-find-all = Visi atitikmenys
btn-replace-all = Pakeisti visus
btn-replace-one = Pakeisti kitą
editor-replace-title = Pakeisti
editor-replace-style = Išlaikyti stilių (foo → bar, Foo → Bar, FOO → BAR)
editor-replace-groups = Reguliariajame reiškinyje $1 ir { "${name}" } yra tai, ką pagavo grupė, $$ — dolerio ženklas
editor-matches-title = Eilutės su `{ $query }`: { $count }
editor-matches-hint = Rašykite, kad susiaurintumėte · Enter: eiti į eilutę · Esc: atgal


# sorting and extended attributes (0.41)
opt-sort-case = Didžiosios ir mažosios raidės rūšiuojant
opt-sort-case-auto = kaip daro ši sistema
opt-sort-case-yes = skirti
opt-sort-case-no = viena abėcėlė
copy-keep-xattrs = Išsaugoti išplėstinius atributus

# editor conversions, disk diff and guides (0.41)
editor-convert-title = Rašyti failą kaip
editor-convert-hint = Rašykite, kad susiaurintumėte · Enter: konvertuoti (F2 įrašo, Ctrl+Z atšaukia) · Esc: atgal
editor-convert-eol = Eilutės pabaiga: { $eol }
editor-convert-bom-on = Baitų tvarkos žymė (BOM): rašoma
editor-convert-bom-off = Baitų tvarkos žymė (BOM): nerašoma
editor-convert-page = Koduotė: { $page }
editor-converted-eol = { $name }: kitą kartą išsaugant eilutės baigsis { $what }
editor-converted-bom-on = { $name }: kitas išsaugojimas įrašys baitų tvarkos žymę
editor-converted-bom-off = { $name }: kitas išsaugojimas baitų tvarkos žymės neįrašys
editor-converted-page = { $name }: kitas išsaugojimas įrašys { $what }
editor-convert-same = Failas jau taip įrašytas
editor-convert-no-bom = { $page } neturi baitų tvarkos žymės: ją turi tik UTF-8 ir UTF-16
editor-convert-lossy = { $page } negali sutalpinti visų { $name } simbolių: niekas nepakeista
btn-reload = Įkelti iš naujo
btn-what-changed = Kas pasikeitė
editor-diff-buffer = redaktoriuje
editor-diff-hint = Šis palyginimas tik parodo, kas pasikeitė — Esc grąžina prie klausimo


# job journal and saved syncs (0.41)
journal-left = Praėjusio seanso nebaigti kopijavimai: { $count } · Alt+Shift+F6 juos parodo
journal-title = Nebaigtos užduotys
journal-hint = Enter tęsti · Del atmesti · Esc uždaryti
journal-none = Nebaigtų užduočių neliko
journal-row = { $kind }: { $what } → { $target } · atlikta failų: { $done }
journal-row-unreadable = nepavyksta perskaityti: { $file } ({ $error })
journal-unreadable = { $file }: tai ne užduoties žurnalas, kurį ši versija moka skaityti ({ $error }). Alt+Shift+F6 jį parodo, o Del ten jį pašalina
journal-resumed = Tęsiama { $kind }: failų jau buvo atlikta { $done }
journal-discarded = Nebaigta užduotis atmesta kartu su tuo, ką ji buvo pusiau įrašiusi
journal-cannot-resume = Šio žurnalo perskaityti nepavyksta; Del jį pašalina
menu-jobs-left = Nebaigtos užduotys
sync-report = Įrašyti vykdymo ataskaitą į (failą)
sync-name = Išsaugoti šią porą kaip (vardas)
sync-saved-button = Išsaugotieji…
sync-saved-title = Išsaugoti sinchronizavimai
sync-saved-hint = Enter vykdyti · F4 keisti · Del pamiršti · Esc uždaryti
sync-saved-none = Dar neišsaugotas joks sinchronizavimas: suteikite jam vardą dialoge (Ctrl+Shift+Y)
sync-saved-removed = { $name } nebėra išsaugotas
sync-report-local = Ataskaita turi būti failas šiame kompiuteryje
sync-report-written = Sinchronizavimo ataskaita yra { $path }
sync-report-not-written = Ataskaitos nepavyko įrašyti į { $path }: { $error }
sync-report-title = Sinchronizavimo ataskaita
sync-report-name = Vardas: { $name }
sync-report-left = Kairė: { $path }
sync-report-right = Dešinė: { $path }
sync-report-to-right = Nukopijuota iš kairės → į dešinę: { $count }
sync-report-to-left = Nukopijuota iš dešinės → į kairę: { $count }
sync-report-cut-right = Pašalinta dešinėje: { $count }
sync-report-cut-left = Pašalinta kairėje: { $count }
sync-report-skipped = Palikta, nors skiriasi: { $count }
sync-report-failed = Nepavyko: { $count }
sync-report-stopped = Sustabdyta pusiaukelėje, todėl šie galėjo būti atlikti arba ne: { $count }
sync-report-not-done = Neatlikta, nes ankstesnis žingsnis nepavyko arba buvo sustabdytas: { $count }
menu-sync-saved = Išsaugoti sinchronizavimai

# hex editor search
hex-search-wrapped = `{ $query }`: rasta pradėjus iš naujo nuo kito failo galo
hex-replace-title = Pakeisti
hex-replace-with = Pakeisti į
btn-replace = Pakeisti
hex-replace-ask = Pakeisti { $was } ties { $at } į { $with }?
hex-replace-length = pakaitas yra { $with } baitų ilgio, o atitikmuo — { $was }: tai pastumtų viską, kas toliau — pirmiausia spauskite Ins, kad įjungtumėte įterpimo režimą

# pixels and video (0.41)
opt-viewer-pixels = Paveikslėliai
opt-viewer-pixels-auto = tikri pikseliai ten, kur terminalas juos piešia
opt-viewer-pixels-kitty = kitty grafikos protokolas
opt-viewer-pixels-iterm = iTerm2 įterptieji vaizdai
opt-viewer-pixels-sixel = sixel
opt-viewer-pixels-off = visada pusiniai blokai
video-needs-program = tai vaizdo įrašas ({ $what }): jo kadrai piešiami per `{ $program }`, kuri šiame kompiuteryje neįdiegta — Shift+F11 vis tiek perskaito konteinerį
video-no-frame = iš šio vaizdo įrašo kadro negauta: { $why }
video-playing = rodoma { $fps } kadrų per sekundę greičiu, be garso — Space pristabdo
video-paused = pristabdyta ties { $at } — Space tęsia, PgUp ir PgDn žengia po dešimtadalį
video-ended = vaizdo įrašas baigėsi — Space rodo iš naujo


# finding things
menu-saved-searches = Išsaugotos paieškos
drive-searches = Išsaugotos paieškos
find-keys = F2 išsaugo šią paiešką vardu · F9 išsaugotosios
find-save-name = Išsaugoti šią paiešką vardu
find-saved = paieška išsaugota vardu { $name }
find-saved-none = dar neišsaugota jokia paieška: F2 paieškos dialoge ją išsaugo
find-saved-title = Išsaugotos paieškos — Enter atveria, F4 redaguoja, Del pamiršta
find-saved-running = ieškoma { $name }…
find-saved-gone = paieška { $name } pamiršta
find-loaded = paieška { $name } įkelta
find-fuzzy = Apytikslis vardas: aukščiau esančios raidės iš eilės, geriausias atitikmuo pirmas
find-meta = Metaduomenys (artist=bowie; camera=)
find-index = Klausti sistemos indekso, o ne vaikščioti po katalogus (naujų failų gali trūkti)
tools-for-index = indeksas, kurio Alt+F7 gali paklausti vietoj vaikščiojimo po katalogus
menu-frecent = Dažni katalogai
frecent-none = dar neaplankytas joks katalogas
frecent-gone = { $path } išimtas iš dažnų katalogų
frecent-hint = rašykite kelio raides · Enter eiti ten · Del pamiršti · Esc uždaryti

# ai third round
ai-redact-off = Maskavimas išjungtas: viskas siunčiama taip, kaip yra
ai-redact-none = Nerasta paslapčių, kurias reikėtų užmaskuoti
ai-redact-count = Prieš siunčiant užmaskuota paslapčių: { $count } ({ $kinds })
ai-redacted = užmaskuota paslapčių: { $count }
ai-preview-title = Kas siunčiama į { $name } ({ $host })
ai-preview-instruction = Nurodymas
ai-preview-earlier = Ligšiolinis pokalbis
ai-preview-question = Klausimas ir jo failai
ai-preview-key = F3 parodo, kas siunčiama
ai-preview-reading = Skaitoma, kas būtų išsiųsta…
key-ai-redact = Maskuot
ai-redact-now-on = Maskavimas įjungtas: raktai, žetonai, privatūs raktai ir slaptažodžiai užmaskuojami prieš ką nors siunčiant
ai-redact-now-off = Maskavimas išjungtas: tai, kas siunčiama, keliauja taip, kaip yra
ai-panel-redact-on = paslaptys maskuojamos
ai-panel-redact-off = paslaptys nemaskuojamos
key-ai-local = Vietin
ai-row-refused = ne šis kompiuteris: atmesta
ai-panel-local-only = tik vietinės
ai-local-now-on = „Tik vietinės“ įjungta: paslauga, kuri yra ne šiame kompiuteryje, atmetama ir jai niekas nesiunčiama
ai-local-now-off = „Tik vietinės“ išjungta: galima klausti bet kurios sąrašo paslaugos
ai-local-refused = „Tik vietinės“ įjungta: { $name } atsako adresu { $host }, o tai ne šis kompiuteris — niekas neišsiųsta (F6 DI lange tai išjungia)
ai-rename-prompt = Kuo turėtų tapti { $count } vardų? (žodžiais)
ai-send-names = Nurodymas ir { $count } iš { $total } vardų — tik vardai, nieko iš failų — siunčiami į { $host }.
ai-rename-ready = modelio šablonas yra dialoge: peržiūrėkite, pataisykite, kas negerai, ir pervadins tik „Gerai“
ai-rename-waiting = modelio pervadinimo šablonas paruoštas: Ctrl+Shift+F6 atveria dialogą su juo
ai-rename-unreadable = modelis neatsakė pervadinimo šablonu: jo atsakymas yra atsakymų kataloge (ai:), niekas nepervadinta
ai-rewrite-prompt = Kuo turėtų tapti pažymėtas blokas? (žodžiais)
ai-rewrite-no-block = pirmiausia pažymėkite bloką (Shift+rodyklės): perrašymas pakeičia būtent tai, kas pažymėta, o stulpelinis blokas neturi formos, į kurią būtų galima grįžti
ai-rewrite-read-only = šis failas čia tik skaitymui: perrašymui nebūtų kur dėtis
ai-rewrite-before = blokas
ai-rewrite-after = kaip jį perrašė modelis
ai-rewrite-hint = Modelio perrašymas greta bloko — Enter įdeda jį vietoj bloko, Esc palieka bloką, kaip yra
diff-hint-rewrite = Enter pakeisti bloką · Esc palikti
ai-rewrite-applied = blokas pakeistas modelio perrašymu — vienas Ctrl+Z jį grąžina
ai-rewrite-left = blokas paliktas, kaip yra
ai-rewrite-stale = blokas nebe toks, koks buvo prašant perrašymo: niekas nepakeista — modelio versija yra atsakymų kataloge (ai:)
ai-rewrite-same = modelis grąžino bloką nepakeistą: nėra ko keisti
ai-answer-kept = atsakymas yra { $name } atsakymų kataloge (ai:) — redaktorius liko priekyje
ai-failure-question = Ši komanda baigėsi kodu { $code }: `{ $command }` — kodėl ji nepavyko ir kas tai pataisytų? Trumpai.
ai-send-failure = Nepavykusi komanda ir paskutinės { $lines } jos išvesties eilutės siunčiamos į { $host }.

# archives and jobs
arc-test-whole = { $name }: patikrinta įrašų { $files }, { $bytes } — visi sveiki
arc-test-bad-title = { $name }: { $bad } iš { $files } įrašų išėjo nesveiki

# extending it
plugin-column-failed = { $column }: { $error } — jo stulpelis lieka tuščias failams, dėl kurių jis neatsakė
plugins-card-columns = Stulpeliai (kaip juos vadina [[column_set]]):
usermenu-far-row = Kas iš FarMenu.ini perskaityta kitaip ({ $count })
usermenu-far-notes = meniu paimtas iš FAR failo FarMenu.ini: kitaip perskaityta dalykų: { $count } — paskutinė eilutė juos išvardija
usermenu-far-not-run = Niekas nebuvo paleista.
find-age = Pakeista prieš tiek dienų (7, >=30, 7-30)
find-bad-age = `{ $text }` nėra amžius dienomis — { $error }
find-label-age = { $days } d.
find-keys-ai = Ctrl+Alt+Q leidžia paiešką nusakyti žodžiais: kalbos modelis užpildo šias eilutes
ai-find-prompt = Ko ieškote? (žodžiais)
ai-send-words = Šie žodžiai — ir nieko iš langų: nei vardo, nei kelio, nei failo — siunčiami į { $host }.
ai-find-ready = modelio paieška yra dialoge: perskaitykite eilutes, pataisykite, kas negerai, ir ieškos tik „Gerai“
ai-find-waiting = modelio paieška paruošta: Alt+F7 atveria dialogą su ja
ai-find-unreadable = modelis neatsakė paieška: jo atsakymas yra atsakymų kataloge (ai:), nieko neieškota
ai-find-left-out = { $said } — eilutėse to NĖRA, nes jos negali to išreikšti: { $what }
# treemap and compare
usage-map-title = Disko naudojimas, visas medis: { $path }
usage-map-walking = Matuojama… failų: { $files }, kol kas { $size }
usage-map-where = { $path } · { $size } · failų: { $files }
usage-map-unreadable = neperskaitytų katalogų: { $count }
usage-map-rest = mažesnių failų: { $count }
usage-map-others = kitų: { $count }
usage-map-empty = Niekas šiame kataloge neužima vietos
usage-map-chosen = { $name } · { $size } · { $share }%
usage-map-hint = ←↑↓→ · Enter į vidų · BkSp aukštyn · Tab sąrašas · G langas ten · Esc
usage-map-hint-list = ↑↓ · Enter į vidų · BkSp aukštyn · Tab žemėlapis · G langas ten · Esc
usage-map-hint-walking = Esc sustabdo
usage-map-top = Tai katalogas, kuris buvo matuotas: virš jo čia nieko nėra
usage-map-no-file = Ši eilutė yra daug mažų failų, o ne vienas, į kurį langas galėtų pereiti
bin-comparing = Abu failai lyginami baitas po baito… Esc sustabdo
bin-bytes = baitų: { $count }
bin-same = Abu failai vienodi baitas į baitą: kiekviename po { $bytes } baitų
bin-where = skirtumas { $at } iš { $count } · pirmas ties poslinkiu { $offset } · besiskiriančių baitų: { $differing }
bin-hint = Alt+↑↓ skirtumai · ↑↓ PgUp PgDn Home End eilutės · Esc uždaryti
diff-unified-on = Vienas stulpelis: kas dingo, tada kas atsirado
diff-unified-off = Du stulpeliai greta
diff-patch-title = Išsaugoti skirtumus kaip pataisą (.diff)
diff-patch-saved = { $name } įrašytas: bendras diff, eilučių: { $lines }
diff-patch-nothing = Abu failai vienodi: nėra pataisos, kurią būtų galima išsaugoti
# wastebasket and external editor
drive-trash = Šiukšlinė
drive-trash-closed = Šiukšlinė (ši sistema neleidžia programai jos išvardyti)
menu-trash = Šiukšlinė
menu-trash-empty = Ištuštinti šiukšlinę
key-trash-restore = Atkurti
key-trash-empty = Tuštint
btn-empty = Ištuštinti
btn-rename = Pervadinti
trash-closed = šios sistemos šiukšlinę galima atverti tik iš jos pačios failų tvarkytuvės
trash-empty = šiukšlinė tuščia
trash-failed = šiukšlinė: { $why }
trash-restored =
    { $count ->
        [one] { $count } elementas grąžintas, kur buvo
        [few] { $count } elementai grąžinti, kur buvo
       *[other] { $count } elementų grąžinta, kur buvo
    }
trash-purged =
    { $count ->
        [one] { $count } elementas ištrintas visam laikui
        [few] { $count } elementai ištrinti visam laikui
       *[other] { $count } elementų ištrinta visam laikui
    }
trash-card-name = Vardas
trash-card-from = Buvo
trash-card-deleted = Ištrinta
trash-card-size = Dydis
trash-clash-title = Atkurti
trash-clash-body = { $path } jau yra.
trash-clash-note = „Perrašyti“ nusiunčia ten esantį į šiukšlinę; „Pervadinti“ grąžina šį šalia jo.
trash-clash-more =
    { $count ->
        [one] Dar { $count } elementas yra tokioje pat padėtyje.
        [few] Dar { $count } elementai yra tokioje pat padėtyje.
       *[other] Dar { $count } elementų yra tokioje pat padėtyje.
    }
trash-purge-title = Ištrinti visam laikui
trash-purge-body =
    { $count ->
        [1] Ar norite ištrinti { $name } iš šiukšlinės?
        [one] Ar norite ištrinti { $count } elementą iš šiukšlinės?
        [few] Ar norite ištrinti { $count } elementus iš šiukšlinės?
       *[other] Ar norite ištrinti { $count } elementų iš šiukšlinės?
    }
trash-purge-note = Tai ištrina visam laikui: niekas jų nebegrąžins.
trash-empty-title = Ištuštinti šiukšlinę
trash-empty-body = Ištrinti viską, kas yra šiukšlinėje?
opt-editor-external = Išorinis redaktorius
opt-editor-use-external = Naudoti išorinį redaktorių klavišui F4 (tada Alt+F4 atveria vidinį)
opt-viewer-external = Išorinė peržiūros programa
opt-viewer-use-external = Naudoti išorinę peržiūros programą klavišui F3 (tada Alt+F3 atveria vidinę)


# editor features
opt-editor-line-numbers = Numeruoti eilutes
editor-line-numbers-on = Eilučių numeriai įjungti
editor-line-numbers-off = Eilučių numeriai išjungti
editor-prompt-pipe-block = Praleisti bloką per komandą:
editor-prompt-pipe-file = Praleisti visą failą per komandą:
editor-prompt-insert-output = Įterpti tai, ką išveda ši komanda:
editor-prompt-insert-file = Įterpti failą:
editor-prompt-write-block = Įrašyti bloką į failą:
editor-pipe-running = Vykdoma { $command } …
editor-pipe-done = { $command }: pakeista tuo, ką ji išvedė, eilučių: { $lines } (Ctrl+Z atšaukia)
editor-pipe-inserted = { $command }: įterpta eilučių: { $lines }
editor-pipe-empty = { $command } nieko neišvedė: niekas neįterpta
editor-pipe-failed = { $command }: { $error } — niekas nepakeista
editor-pipe-changed = { $command }: tekstas pasikeitė, kol komanda buvo vykdoma — niekas nepakeista
editor-pipe-gone = { $command }: failas, kuriam ji buvo paleista, nebeatvertas — niekas nepakeista
editor-pipe-column = Stulpelinio bloko per komandą praleisti negalima: pažymėkite ištisas eilutes arba nieko — visam failui
editor-file-inserted = { $name } įterptas, eilučių: { $lines }
editor-insert-not-local = Į redaktorių įskaityti ir iš jo įrašyti galima tik šio kompiuterio failą
editor-insert-too-big = per didelis įskaityti į redaktorių
editor-block-written = Blokas įrašytas į { $name }: { $size }
editor-block-write-self = { $name } yra redaguojamas failas: blokas ant jo neįrašytas
editor-choice-hint = Rašykite, kad susiaurintumėte · Enter: rinktis · Esc: atgal
editor-syntax-title = Spalvinti failą kaip
editor-syntax-auto = Ką sako jo vardas: { $language }
editor-syntax-auto-none = Ką sako jo vardas: nieko žinomo
editor-syntax-none = Paprastas tekstas, be spalvų
editor-syntax-set = { $name } dabar skaitomas kaip { $language }
editor-syntax-off = { $name } dabar skaitomas kaip paprastas tekstas
editor-case-heading = Keisti raidžių dydį
editor-case-upper = DIDŽIOSIOS RAIDĖS
editor-case-lower = mažosios raidės
editor-case-title = Antraštinės Raidės: didžioji kiekvieno žodžio pradžioje
editor-case-toggle = aPVERSTI: didžiosios tampa mažosiomis, o mažosios — didžiosiomis
editor-no-word = Nieko nepažymėta, ir žymeklis nėra ant žodžio
editor-prompt-isearch = Paieška rašant (↓ ↑ kitas ir ankstesnis, Esc grįžta)
editor-word-only = Žodis po žymekliu niekur kitur neparašytas
editor-not-on-word = Žymeklis nėra ant žodžio
editor-prompt-reflow = Į kiek stulpelių išdėstyti pastraipą? (vien Enter: { $width })
editor-reflowed = Pastraipa išdėstyta į { $width } stulpelių (Ctrl+Z atšaukia)
editor-reflow-bad-width = { $width } nėra plotis: reikia stulpelių skaičiaus nuo 8 iki 1000
editor-no-paragraph = Žymeklis yra tuščioje eilutėje: pastraipos čia nėra
editor-no-name-yet = Šis failas dar neturi vardo: pirmiausia jį išsaugokite
editor-tools-title = Redaktoriaus komandos
editor-tool-isearch = Paieška rašant
editor-tool-word-next = Kita vieta, kur parašytas žodis po žymekliu
editor-tool-word-prev = Ankstesnė vieta, kur parašytas žodis po žymekliu
editor-tool-pipe = Praleisti bloką arba visą failą per komandą
editor-tool-write-block = Įrašyti bloką į failą
editor-tool-case = Keisti bloko raidžių dydį
editor-tool-reflow = Iš naujo išdėstyti pastraipą pagal plotį
editor-tool-insert-file = Įterpti failą ties žymekliu
editor-tool-insert-output = Įterpti tai, ką išveda komanda
editor-tool-insert-date = Įterpti datą ir laiką
editor-tool-insert-name = Įterpti šio failo vardą
editor-tool-insert-path = Įterpti visą šio failo kelią
editor-tool-line-numbers = Įjungti arba išjungti eilučių numerius
editor-tool-syntax = Pasirinkti kalbą, pagal kurią spalvinamas failas
