key-help = Ayuda
key-menu = Menú
key-view = Ver
key-edit = Editar
key-copy = Copiar
key-move = Mover
key-mkdir = CrDIR
key-delete = Borrar
key-pulldown = BarMnú
key-quit = Salir
# The key bar while a modifier is held: FAR's own words, six columns wide.
key-add = Añadir
key-extract = Extraer
key-arccmd = ArcCmd
key-edit-new = Editar…
key-rename = Renomb
key-group = Grupo
key-selup = SelArr
key-left = Izqda
key-right = Dcha
key-sort-name = Nombre
key-sort-ext = Extens
key-sort-write = Escrit
key-sort-size = Tamaño
key-sort-unsort = Desord
key-sort-create = Creac
key-sort-access = Acceso
key-sort = Ordenar
key-mklink = Enlace
key-find = Buscar
key-history = Histor
key-folders = CarpHs
key-plugins = Plugin

list-up = UP-DIR
list-folder = Directorio
col-name = Nombre
col-ext = Ext
col-size = Tamaño
col-allocated = Ocupado
col-modified = Modificado
col-created = Creado
col-accessed = Acceso
col-attr = Atr
col-owner = Propietario
col-group = Grupo
col-descr = Descripción
col-links = Enl
col-git = Git
descr-title = Descripción de { $name }
descr-set = { $name } descrito
descr-cleared = descripción de { $name } quitada
descr-not-here = las descripciones viven junto a una carpeta de esta máquina
col-date-time = Fecha       Hora
panel-list-count = { $count } elementos en la lista
panel-empty = (vacío)
panel-loading = leyendo…

prompt-select-mask = Seleccionar archivos por máscara:
prompt-deselect-mask = Deseleccionar archivos por máscara:
prompt-goto = Ir al directorio:
prompt-filter = Máscara o parte de un nombre:
prompt-search = Búsqueda rápida:

dialog-error = Error
dialog-help-hint = Esc vuelve al índice · F7 busca en todos los capítulos · F1 cierra · ↑↓ PgArr PgAb para desplazar
dialog-help-index = FinalCommander { $version } — Ayuda
dialog-help-hint-index = Entrar abre un capítulo · F7 busca en todos los capítulos · Esc cierra
help-search-prompt = Buscar en la ayuda
help-search-none = Ninguna línea de la ayuda contiene «{ $query }»
help-search-title = «{ $query }» en la ayuda: { $count } líneas
output-search-prompt = Buscar en la salida
output-search-empty = Todavía no se ha impreso nada
output-search-found = Línea { $line } de { $lines }
output-search-wrapped = Línea { $line } de { $lines }, otra vez desde la más reciente
output-search-none = Ninguna línea de la salida contiene «{ $query }»

msg-not-implemented = { $command } aún no está implementado en esta versión
msg-selected = { $count } elementos seleccionados
msg-deselected = { $count } elementos deseleccionados
msg-mask-invalid = máscara no válida: { $error }
msg-theme = tema: { $name }
msg-font-bigger = fuente: se ha pedido una mayor al terminal
msg-font-smaller = fuente: se ha pedido una menor al terminal
msg-font-reset = fuente: se ha pedido la inicial al terminal
msg-sort = orden: { $key }{ $reverse }
msg-sort-groups-on = grupos de orden activos ({ $count })
msg-sort-groups-off = grupos de orden desactivados ({ $count })
msg-sort-groups-none = no hay grupos de orden definidos: mira [[sort_group]] en la configuración
msg-view = vista: { $mode }
msg-hidden-on = archivos ocultos visibles
msg-hidden-off = archivos ocultos no visibles
msg-no-history = no hay más historial
msg-tab-last = un panel siempre tiene una pestaña
msg-tab-end = la pestaña ya está en ese extremo de la fila
tabs-title = Pestañas
msg-tab-moved = pestaña movida al otro panel
msg-tab-locked = esta pestaña está fijada a su directorio
msg-tab-locked-on = pestaña fijada a su directorio
msg-tab-locked-off = pestaña liberada
msg-bookmark-set = atajo { $digit }: { $path }
msg-bookmark-unset = el atajo { $digit } no está definido
msg-bookmark-added = { $path } guardada
msg-bookmark-removed = { $path } quitada
msg-bookmark-known = esa carpeta ya está en la lista
msg-dirsize = { $path }: { $bytes } en { $files } archivos
msg-dirsize-many = midiendo { $count } carpetas…
usage-title = Uso del disco: { $path } · { $size }
usage-not-here = el uso de una carpeta se mide en un disco, no dentro de un archivo comprimido ni en una lista de archivos encontrados
msg-dirsize-no-folder = aquí no hay ninguna carpeta que medir

prompt-copy-to = Copiar { $what } a:
prompt-move-to = Mover/renombrar { $what } a:
prompt-mkdir = Crear el directorio:
prompt-tab-name = Nombre de la pestaña:
prompt-bookmark-name = Nombre de la carpeta:
what-selected = { $count } elementos seleccionados
confirm-delete-title = Borrar
confirm-delete-body =
    { $count ->
        [one] ¿Borrar { $name } ({ $bytes })?
       *[other] ¿Borrar { $count } elementos ({ $bytes })?
    }
prompt-link = Crear un enlace a { $name } como:
link-created = { $name } creado
confirm-wipe-title = Borrado seguro
confirm-wipe-body =
    { $count ->
        [one] ¿Sobrescribir y eliminar { $name } ({ $bytes })?
       *[other] ¿Sobrescribir y eliminar { $count } elementos ({ $bytes })?
    }
confirm-wipe-honest = Los bytes se sobrescriben una vez y luego se desvincula el archivo. En un SSD, en un sistema de archivos copy-on-write o con instantáneas pueden sobrevivir copias anteriores — y el nombre sobrevive en el directorio y en el journal.
job-copy = Copia
job-move = Movimiento
job-delete = Borrado
job-wipe = Borrado seguro
job-trash = Moviendo a la papelera
job-replace = Sustituyendo
job-pack = Comprimiendo
job-rebuild = Escribiendo el archivo comprimido

# ── the attributes dialog (SPEC 10 §3.8) ───────────────────────────────────────────────
attr-title = Atributos del archivo
attr-count = { $count } elementos seleccionados
attr-now = ahora { $text } ({ $octal })
attr-owner-read = El propietario puede leer
attr-owner-write = El propietario puede escribir
attr-owner-exec = El propietario puede ejecutar
attr-group-read = El grupo puede leer
attr-group-write = El grupo puede escribir
attr-group-exec = El grupo puede ejecutar
attr-other-read = Todos pueden leer
attr-other-write = Todos pueden escribir
attr-other-exec = Todos pueden ejecutar
attr-setuid = Ejecutar como el propietario (setuid)
attr-setgid = Ejecutar como el grupo (setgid)
attr-sticky = Solo el propietario puede borrar (sticky)
attr-readonly = Solo lectura
attr-hidden = Oculto
attr-system = Sistema
attr-archive = Archivo
attr-temporary = Temporal
attr-offline = Sin conexión
attr-indexed = El indexador puede leerlo
attr-compressed = comprimido
attr-encrypted = cifrado
attr-sparse = disperso
attr-reparse = un punto de reanálisis
attr-also = además { $what }
attr-mtime = Modificado
attr-atime = Última lectura
attr-btime = Creado
attr-ctime = Registro cambiado
attr-btime-readonly = (la fecha de creación solo puede fijarse en Windows)
attr-utc = Horas en UTC
attr-owner = Propietario (usuario:grupo)
attr-owner-is = propietario { $who }
btn-attr-original = Original
btn-attr-current = Ahora
btn-attr-blank = Vacío
attr-recursive = Y todo lo que hay en las carpetas
attr-bad-time = `{ $text }` no es una fecha y hora (AAAA-MM-DD HH:MM:SS)
attr-bad-owner = aquí ningún usuario ni grupo se llama `{ $text }`
attr-in-archive = cambiar esto vuelve a escribir el archivo, con las demás entradas como están
job-attrs = Ajustando atributos

# ── the copy dialog (SPEC 03 §4) ───────────────────────────────────────────────────────
copy-title = Copiar
copy-title-move = Renombrar o mover
copy-target = A
copy-conflict = Si ya está
copy-conflict-ask = preguntar
copy-conflict-overwrite = sobrescribir
copy-conflict-newer = sobrescribir si el origen es más nuevo
copy-conflict-skip = omitir
copy-conflict-rename = dar a la copia un nombre libre
copy-conflict-append = añadir al final
copy-only-newer = Solo los archivos más nuevos que los que hay
copy-filter = Solo los nombres que coincidan
copy-follow-links = Copiar a lo que apunta el enlace, no el enlace
copy-keep-times = Conservar las fechas
copy-keep-mode = Conservar los permisos
copy-verify = Releer cada archivo y compararlo
copy-several = { $count } destinos, uno tras otro

# ── the terminal (SPEC 06 §4) ──────────────────────────────────────────────────────────
term-started = un terminal con { $shell }
term-hint = Ctrl+Alt+E: volver a los paneles
term-ended = ha terminado — una tecla lo cierra
job-mkdir = Creación de directorio
job-planning = analizando…
job-counters = { $files } / { $filestotal } archivos · { $bytes } / { $bytestotal }
job-rates = { $speed }/s · { $elapsed } transcurridos · faltan { $eta }
job-rates-no-eta = { $speed }/s · { $elapsed } transcurridos
job-hint = Esc cancela
job-hint-queue = Esc cancela · B segundo plano
job-queued = { $kind } está en la cola, con otros { $ahead } trabajos sin terminar · Alt+Shift+F5 los lista
job-background = { $kind } sigue en segundo plano · Alt+Shift+F5 lista los trabajos
job-asks = { $kind } espera una respuesta · Alt+Shift+F5
job-paused = { $kind } en pausa
job-resumed = { $kind } continúa
job-retry-nothing = Nada que reintentar
job-still-waiting = Ese trabajo espera su turno
jobs-title = Trabajos ({ $active } sin terminar)
jobs-hint = Intro mostrar · Espacio pausa · Supr detener · F5 reintentar · Ctrl+↑↓ orden · Esc cerrar
jobs-quit-title = Hay trabajos en curso
jobs-quit-body = Estos trabajos se detienen si el programa termina ahora:
job-state-running = en curso
job-state-paused = en pausa
job-state-asking = pregunta
job-state-queued = en espera
job-state-held = retenido
job-state-done = terminado
job-state-failed = errores
job-state-cancelled = cancelado
job-done = { $kind }: { $files } archivos, { $bytes }
job-done-renamed = { $kind }: { $files } archivos movidos sin copiar
job-done-skipped = , { $skipped } omitidos
job-done-verified = , { $verified } verificados
job-done-unverified = , { $unverified } no se pueden releer aquí
job-done-errors = , { $errors } errores
job-cancelled = { $kind } cancelada
job-errors-title = Errores
conflict-title = El destino ya existe
conflict-dst = destino  { $path }   { $bytes }
conflict-src = origen   { $path }   { $bytes }
conflict-newer = el origen es más reciente
conflict-older = el destino es más reciente
btn-overwrite = Sobrescribir
btn-overwrite-all = Todos
btn-skip = Omitir
btn-skip-all = Omitir todos
btn-retry = Reintentar
btn-move = Mover
btn-delete = Borrar
btn-wipe = Borrar a fondo
btn-copy = Copiar
btn-discard = Descartar
btn-kill = Terminar
btn-quit = Salir
btn-play = Reproducir
btn-run = Ejecutar
btn-accept = Aceptar
btn-cancel = Cancelar
msg-nothing-to-do = nada seleccionado
cmd-complete-nothing = no hay nada con que completarlo
complete-several = { $count } nombres empiezan así
form-moving = las flechas mueven el diálogo · Intro o Esc lo deja ahí
form-moved = el diálogo se queda donde lo pusiste
msg-trash-refused = La papelera no lo acepta — Mayús+Supr lo borra definitivamente.

# Checksums (SPEC 03 §9)
sum-title = Sumas de verificación
sum-algorithm = Hash
sum-one-file = Una sola lista para todos
sum-name = Nombre
sum-working = { $name }: { $done } de { $total }
sum-verifying = comprobando con { $name }
sum-stopped = detenido
sum-written = { $name } escrito
sum-no-name = la lista necesita un nombre
sum-local-only = una suma solo puede leer un archivo de esta máquina
sum-all-match = todos { $count } coinciden
sum-verify-title = Sumas de verificación
sum-verify-summary = { $ok } coinciden, { $bad } no
sum-differs = no coincide
sum-missing = no está
sum-unreadable = no se puede leer

# Synchronising two directories (SPEC 03 §10)
sync-title = Sincronizar
sync-between = { $left }  ↔  { $right }
sync-recursive = También todo lo de los subdirectorios
sync-both-ways = En los dos sentidos, gana el más reciente
sync-delete-extra = Ofrecer quitar lo que el otro lado no tiene
sync-by-content = Comparar los bytes, no el tamaño y la fecha
sync-filter = Solo los nombres que coinciden
sync-walking = mirando { $at } — { $pairs } hasta ahora
sync-planned = { $copies } por copiar ({ $bytes }), { $deletes } por quitar
sync-hint = Espacio siguiente respuesta · ← → sentido · = dejar · F2 hacerlo · Esc cancelar
sync-hint-walking = Esc detiene el recorrido
sync-nothing-to-do = todas las filas dicen dejar las cosas como están
sync-local-only = una sincronización solo puede leer y escribir en esta máquina
sync-same-directory = los dos paneles están en el mismo directorio

# What to run for a kind of file (SPEC 06 §5)
assoc-none = nada dice cómo abrir { $name }
usermenu-empty = Aún no hay menú del usuario: escríbelo en { $path }
usermenu-last-gone = la entrada elegida la última vez ya no está en este menú
usermenu-bad-condition = Condición no reconocida — { $problem }
assoc-desktop = Entregarlo al escritorio
assoc-open-with = Abrir { $name } con
assoc-no-pager = ninguna asociación ni $PAGER dicen con qué leerlo
assoc-no-editor = ninguna asociación ni $VISUAL dicen con qué escribirlo

key-wrap = Ajuste
key-hex = Hex
key-text = Texto
key-dump = Volcado
key-doc = Doc
key-search = Buscar
key-code = Codif
key-follow = Seguir
viewer-prompt-search = Buscar ({ $mode }):
viewer-prompt-goto = Ir al desplazamiento, o NN% :
viewer-not-found = `{ $query }` no encontrado
viewer-follow-on = siguiendo el final del archivo
viewer-follow-off = seguimiento desactivado
viewer-codepage = codificación: { $name }
viewer-wrap-mode = { $mode }
viewer-no-undo = no hay posición anterior
viewer-empty = (archivo vacío)
viewer-not-markdown = no es un archivo Markdown, o es demasiado grande para maquetarlo
viewer-not-table = no es un archivo tabular, o es demasiado grande para leerlo entero
viewer-base64-done = Descodificado: { $bytes }
viewer-not-base64 = Este archivo no es base64: { $why }
viewer-status = { $mode } { $codepage }  { $offset } / { $len }  { $percent }%

key-disasm = Desens
viewer-disasm-failed = no se puede desensamblar { $name }: { $error }
viewer-decompile-running = { $tool } se está ejecutando sobre { $name }…
viewer-decompile-none = aquí no hay nada que descompile { $format }; instala una herramienta y añade una regla [[decompile]] en fcmd.toml
arc-needs-program = este archivo lo lee `{ $program }`, que no está instalado en esta máquina
viewer-decompile-failed = { $tool }: { $error }
viewer-blame-derived = git blame es del propio archivo: vuelve primero al archivo
viewer-derived = { $producer } de { $name }

key-save = Guardar
key-replace = Reempl
editor-prompt-goto = Ir a la línea, o línea:columna :
editor-prompt-save-as = Guardar como:
editor-saved = { $name }: guardado, { $bytes } bytes
editor-nothing-to-save = { $name } no tiene cambios sin guardar
editor-not-found = `{ $query }` no encontrado
editor-replaced = { $count } reemplazos
editor-read-only = { $name } es de solo lectura
editor-readonly-on = solo lectura: se rechazan los cambios
editor-readonly-off = solo lectura desactivada
editor-no-undo = no queda nada por deshacer
editor-no-redo = nada que rehacer
editor-discard-title = Cambios sin guardar
editor-discard-body = { $name } tiene cambios sin guardar. ¿Salir y perderlos?
editor-changed-title = Cambiado en el disco
editor-changed-body = { $name } cambió en el disco desde que se abrió. ¿Sobrescribirlo?
editor-reloaded = { $name } recargado
editor-failed = { $name }: { $error }

# netbox (SPEC 04 §6)
net-sites-title = Conexiones
net-sites-hint = Entrar conecta · n URL · Ins nueva · F4 editar · d desconecta · Esc cierra
net-no-sites = Aún no hay sitios. Pulsa `n` para escribir una URL, o añade bloques [[site]] a la configuración.
net-not-a-stored-session = esta fila es una conexión abierta, no una sesión guardada
net-site-forgotten = { $name } ya no está guardada
net-prompt-url = Conectar a:
net-prompt-password = Contraseña de { $user }@{ $host }:
net-prompt-passphrase = Frase de contraseña de la clave de { $user }@{ $host }:
net-connecting = conectando a { $host }…
net-connected = conectado a { $host }
net-disconnected = { $host } desconectado
net-not-connected = { $host } no está conectado
net-local-panel = este panel está en un sistema de archivos local
net-nothing-pending = ninguna conexión espera eso
net-hostkey-title = Clave de host desconocida
net-hostkey-body = { $host } no está en known_hosts. Su huella:
net-hostkey-question = ¿Registrarla y conectar?
net-hostkey-changed-title = La clave del host ha CAMBIADO
net-hostkey-changed-body = { $host } respondió con una clave que no es la registrada.
net-hostkey-changed-where = La registrada está en { $path }, línea { $line }.
net-hostkey-changed-what = Un servidor reinstalado se ve igual que alguien en medio. Aquí no se acepta nada: abre el archivo, lee la línea y quítala tú si sabes por qué cambió.
btn-open-file = Abrir el archivo
net-fetching = descargando { $name }…
net-viewing-copy = { $name }: copia local, descargada para el visor
net-too-big = { $name } supera { $limit }: aumenta net.fetch_limit_mb
net-editing = { $name }: editas una copia local; al guardar se reescribe
net-writing-back = reescribiendo { $name }…
net-written-back = { $name }: reescrito, { $bytes } bytes
net-write-back-failed = la reescritura falló: { $error } — tus cambios están en { $local }
net-changed-title = Cambiado en el servidor
net-changed-body = { $name } cambió en el servidor desde que se descargó. ¿Sobrescribirlo?
confirm-delete-permanent = Esto borra definitivamente: para estos no hay papelera.
confirm-trash-title = Mover a la papelera
confirm-trash-body =
    { $count ->
        [one] ¿Mover { $name } ({ $bytes }) a la papelera?
       *[other] ¿Mover { $count } elementos ({ $bytes }) a la papelera?
    }
confirm-trash-where = Van donde el escritorio guarda los archivos borrados, y de ahí se pueden recuperar.

# ---------------------------------------------------------------- barra de menús (SPEC 10)
menu-left = &Izquierda
menu-right = &Derecha
menu-files = &Archivos
menu-commands = &Comandos
menu-options = &Opciones
menu-brief = &Breve
menu-medium = &Medio
menu-full = &Completa
menu-detailed = De&tallada
menu-owners = Propi&etarios
menu-links = En&laces
menu-alt-full = Com&pleta, tamaños enteros
menu-descriptions = &Descripciones
menu-long-descriptions = Descripciones lar&gas
menu-wide = &Ancha
menu-info-panel = Panel de &información
menu-quick-view = &Vista rápida
menu-tree-panel = Panel de &árbol
menu-tab-new = &Nueva pestaña
menu-tab-close = Cerrar la pesta&ña
menu-tab-next = Pestaña siguiente
menu-tab-prev = Pestaña anterior
menu-tab-move = Mover la pestaña al otro panel
menu-tab-lock = Fi&jar la pestaña a su directorio
menu-tab-name = Dar nombre a la pestaña
menu-tab-list = Todas las pestañas
menu-sort-modes = M&odos de orden
menu-hidden = Archivos ocultos &y del sistema
menu-filter = &Filtro de archivos
menu-filter-sets = Filtros con nombre
menu-panel-onoff = Panel &sí/no
menu-reread = Volve&r a leer
menu-change-drive = Cambiar &unidad
menu-view = &Ver
menu-edit = &Editar
menu-edit-new = E&ditar un archivo nuevo
menu-copy = &Copiar
menu-rename = &Renombrar o mover
menu-link = Crear en&lace
menu-multirename = Renombrar se&gún un patrón…
menu-multirename-undo = Deshacer el último renombrado múltiple
menu-mkdir = Crear carpeta
menu-delete = &Borrar
menu-wipe = Borrar de forma seg&ura
menu-checksum = Sumas de veri&ficación…
menu-checksum-verify = Comprobar con una lista
menu-split = Dividir archivo…
menu-combine = Unir archivos
menu-delete-permanent = B&orrar definitivamente
menu-archive-add = &Añadir al archivo
menu-archive-extract = E&xtraer archivos
menu-archive-commands = Co&mandos de archivo
menu-archive-test = Probar arc&hivo
menu-attributes = Atribu&tos del archivo
menu-apply-command = A&plicar comando
menu-replace-in-files = Sustituir en archivos…
menu-select-group = &Seleccionar grupo
menu-select-ext = Seleccionar por esta extensión
menu-unselect-ext = Deseleccionar por esta extensión
menu-unselect-group = Deseleccio&nar grupo
menu-invert-selection = &Invertir la selección
menu-select-all = Seleccionar todo
menu-restore-selection = Restaurar la selecci&ón
menu-dirsize = Tama&ño de la carpeta
menu-find-file = &Buscar archivo
menu-history = &Historial de comandos
menu-view-history = Historial de &vistas y ediciones
menu-folders-history = Historial &de carpetas
menu-usage = Uso del disco
menu-jobs = Cola de traba&jos
menu-branch = Vista de rama
menu-bookmarks = Carpetas favoritas
menu-swap-panels = I&ntercambiar los paneles
menu-sync-panel = &Sincronizar el otro panel
menu-panels-onoff = &Paneles sí/no
menu-compare-folders = &Comparar carpetas
menu-sync-folders = S&incronizar carpetas
menu-compare-contents = Compa&rar por contenido
menu-panelize = Panel con la salida de una orden
menu-view-filtered = Ver a través de una orden
menu-learn-keys = Aprender teclas
menu-tunnel = Abrir &un túnel…
menu-tunnels = T&úneles…
tunnel-title = Túnel: [local:]host:puerto
tunnel-list-title = Túneles
tunnel-none = no hay ningún túnel abierto
tunnel-open = túnel abierto: { $what }
tunnel-closed = túnel cerrado
tunnel-failed = { $what }: { $why }
tunnel-needs-connection = un túnel va por una conexión: abre una con Alt+N
menu-connections = Cone&xiones
menu-disconnect = Desconectar
menu-user-menu = &Menú del usuario
menu-associations = &Asociaciones de archivos
menu-folder-shortcuts = Accesos a carpetas
menu-plugin-commands = C&omandos de los complementos
menu-screens-list = &Lista de pantallas
menu-macro-record = &Grabar una macro
menu-macro-list = Macros
menu-task-list = Lista de &tareas
menu-system-settings = Aju&stes del sistema
menu-panel-settings = Ajustes de los &paneles
menu-interface-settings = Ajustes de la interfa&z
menu-languages = &Idiomas
menu-themes = &Temas
menu-confirmations = Co&nfirmaciones
menu-viewer-settings = Ajustes del &visor
menu-editor-settings = Ajust&es del editor
menu-codepages = Páginas &de códigos
menu-sort-groups = Grupos de &orden
menu-mask-groups = &Máscaras de archivo
menu-settings-all = Todos los aj&ustes
menu-highlighting = &Resaltado de archivos
menu-plugins-config = Configuración de &los complementos
menu-help = A&yuda
menu-save-setup = &Guardar los ajustes
menu-quit = S&alir
opt-natural-sort = Orden natural de los números
opt-allow-reverse = Permitir el orden inverso
opt-select-folders = Las máscaras también seleccionan carpetas
cmd-no-network-name = Una ruta en este disco no tiene nombre de red
opt-trash = Usar la papelera del sistema
opt-system-copy = Usar la copia del sistema
opt-save-history = Mantener el historial entre sesiones
opt-history-lifetime = Duración del historial, en días
opt-forever = para siempre
opt-fetch-limit = Límite de tamaño de archivos remotos
opt-keepalive = Keep-alive de SSH
opt-off = apagado
opt-show-hidden = Mostrar los archivos ocultos y del sistema
opt-column-titles = Mostrar los títulos de columna
opt-status-line = Mostrar la línea de estado
opt-scrollbar = Mostrar la barra de desplazamiento
opt-free-space = Mostrar el espacio libre
opt-auto-refresh = Actualizar el panel automáticamente
opt-dirs-first = Carpetas primero
opt-selected-first = Archivos seleccionados primero
opt-reverse = Orden inverso
opt-view = Vista del panel
opt-sort = Ordenar por
opt-quick-search = Búsqueda rápida
opt-quick-alt = con Alt
opt-quick-plain = con letras
opt-key-bar = Mostrar la barra de teclas
opt-status-bar = Mostrar la barra de estado
opt-syntax = Colorear los lenguajes reconocidos
opt-markdown = Markdown maquetado para leerlo
opt-table = Abrir .csv y .jsonl como tabla
opt-keymap = Mapa de teclas
opt-clock = Reloj en la esquina
opt-mouse = Soporte del ratón
opt-clipboard = Portapapeles a través del terminal (OSC 52)
opt-command-screen = Mostrar la salida del comando mientras se ejecuta
opt-menu-always = Mostrar siempre la barra de menús
opt-save-exit = Guardar la configuración al salir
opt-confirm-delete = Borrado
opt-confirm-overwrite = Sobrescritura
opt-confirm-exit = Salida
opt-confirm-editor = Salir del editor con cambios sin guardar
opt-applied = ajustes aplicados
opt-saved = guardado en { $path }
opt-set-title = { $key }
opt-set = { $key } = { $value }
opt-not-a-toggle = No es un sí o no: Enter escribe un valor
opt-settings-hint = ↑↓ elegir · escribe para filtrar · Enter o F4 valor · Espacio invertir · Supr predeterminado · Esc cerrar
opt-language-set = idioma: { $name }
sort-name = Nombre
sort-ext = Extensión
sort-size = Tamaño
sort-mtime = Fecha
sort-btime = Creación
sort-atime = Último acceso
sort-descr = Descripción
sort-owner = Propietario
sort-git = Estado de git
key-sort-descr = Descr
key-sort-owner = Propiet
sort-none = Sin ordenar
sort-set = orden: { $name }
btn-ok = Aceptar
pick-hint = ↑↓ elegir · escribe para filtrar · Intro aplicar · Esc cerrar
pick-empty = (nada que elegir)
# ------------------------------------------------- los tres historiales (SPEC 06 §6)
hist-pinned = { $line } está fijada · ya nada la desplaza
hist-unpinned = { $line } ya no está fijada
hist-is-pinned = { $line } está fijada · Ins la suelta primero
hist-clear-body = ¿Vaciar este historial de todo lo que no está fijado?
hist-cleared = el historial está vacío salvo lo fijado
history-hint = ↑↓ elegir · Intro aplicar · Ins fijar · Mayús+Supr una · Supr todas · Esc cerrar


# ------------------------------------------------- session editor (SPEC 10 §3.5)
site-new = Nueva conexión
site-edit = Editar la conexión
site-name = Nombre
site-host = Servidor
site-port = Puerto
site-user = Usuario
link-title = Crear un enlace
link-name = El enlace
link-kind = Tipo
link-kind-relative = simbólico, relativo — sobrevive al mover el árbol
link-kind-absolute = simbólico, absoluto — sobrevive al mover el enlace
link-kind-hard = duro — un segundo nombre para el mismo archivo
menu-colors = &Colores…
palette-title = Colores de { $theme }
palette-hint = Intro un color · F2 guardar · Esc cerrar
palette-ask = Un color para { $key }
palette-bad-color = { $value } no es un color (un nombre, 0–255 o #rrggbb)
palette-saved = los colores se escribieron en el tema { $theme }
palette-unsaved = los colores cambiaron solo en la pantalla — F2 los escribe
opt-viewer-status-line = Mostrar la línea superior (nombre y posición)
opt-viewer-scrollbar = Mostrar barra de desplazamiento
opt-viewer-key-bar = Mostrar la barra de teclas
opt-viewer-wrap = Líneas largas
opt-viewer-wrap-off = no ajustar
opt-viewer-wrap-char = ajustar en el borde
opt-viewer-wrap-word = ajustar por palabra
opt-viewer-tab-width = Ancho de tabulación
opt-viewer-remember = Recordar la posición y la codificación de cada archivo
opt-editor-tabs = Tabulaciones
opt-editor-tabs-keep = dejarlas como están
opt-editor-tabs-typed = expandir las que se escriben
opt-editor-tabs-all = expandirlas todas al abrir el archivo
opt-editor-tab-width = Ancho de la tabulación
opt-editor-auto-indent = Mantener la sangría de la línea anterior
opt-editor-whitespace = Mostrar espacios y tabulaciones
opt-editor-cursor-after = El cursor queda después del texto encontrado
opt-editor-beyond-eol = El cursor puede pasar del final de la línea
opt-editor-trim-trailing = Quitar los espacios al final de las líneas al guardar
editor-not-a-character = esa tecla no tiene un carácter que insertar
paste-one-line =
    { $count ->
        [one] solo se pegó la primera línea · una línea más sin pegar
       *[other] solo se pegó la primera línea · { $count } líneas más sin pegar
    }
clip-names =
    { $count ->
        [one] un nombre en el portapapeles
       *[other] { $count } nombres en el portapapeles
    }
clip-paths =
    { $count ->
        [one] una ruta en el portapapeles
       *[other] { $count } rutas en el portapapeles
    }
clip-line = la línea de órdenes está en el portapapeles
clip-selection = lo marcado está en el portapapeles
clip-off = el portapapeles está desactivado — `ui.clipboard` en los ajustes
history-gone = { $name } ya no está
hl-new = Nuevo grupo de color
hl-edit = Grupo de color
# ------------------------------------------------- las diferencias (SPEC 05 §3)
diff-title = Diferencias
menu-diff = Di&ferencias entre dos archivos
diff-needs-two = Pon el cursor sobre un archivo en cada panel
diff-not-here = { $name } no es un archivo en este disco
diff-too-big = { $name } es más grande que { $most }
diff-same = Los dos archivos son iguales
diff-where = bloque { $at } de { $count } · -{ $removed } +{ $added } líneas
diff-no-next = La última diferencia
diff-no-previous = La primera diferencia
diff-hint = Alt+↑↓ bloques · Tab lado · F5 tomar · F2 escribir · W espacios · C mayúsculas · E fin de línea · Esc cerrar
diff-hint-git = Alt+↑↓ bloques · S preparar el bloque · Tab lado · F5 tomar · F2 escribir · Esc cerrar
diff-hint-git-staged = Alt+↑↓ bloques · S quitar el bloque del índice · Tab lado · F5 tomar · F2 escribir · Esc cerrar
diff-stage-edited = el diff tiene fusiones sin escribir: primero F2, luego S
diff-merged-left = El archivo de la izquierda tomó esa diferencia
diff-merged-right = El archivo de la derecha tomó esa diferencia
diff-no-hunk-here = No hay ninguna diferencia arriba de la pantalla — Alt+↓ trae una aquí
diff-nothing-to-write = No se ha tomado ninguna diferencia
diff-written = { $count } escritos
diff-unwritten = Una diferencia tomada no está escrita — F2 la escribe, Esc otra vez la deja
diff-ignore-space-on = Espacios ignorados
diff-ignore-space-off = Los espacios vuelven a contar
diff-ignore-case-on = Mayúsculas ignoradas
diff-ignore-case-off = Las mayúsculas vuelven a contar
diff-ignore-eol-on = Fin de línea ignorado
diff-ignore-eol-off = El fin de línea vuelve a contar

grab-hint = ↑↓←→ mover · Mayús+↑↓←→ marcar · Ctrl+A todo · Enter copiar · Esc cerrar
grab-copied = { $rows } líneas de la pantalla copiadas

# ------------------------------------------------- las páginas de códigos (SPEC 05 §1.12)
cp-title = Páginas de códigos
cp-name-title = Cómo llamar a esta página de códigos
cp-set = Leído como { $name }
menu-panel-names = Página de códigos de los nombres…
names-title = Leer los nombres como
names-system = Como los da el sistema
names-set = Nombres leídos como { $name }
names-system-set = Nombres como los da el sistema
names-not-here = Un nombre en este sistema no son bytes, así que no hay una segunda forma de leerlo
cp-marked = { $name } está entre las que recorre F8
cp-unmarked = { $name } ya no está entre las que recorre F8
cp-hint = ↑↓ elegir · Intro leer así · Ins marcar para F8 · F4 renombrar · Esc cerrar

# ------------------------------------------------- los grupos de máscaras (SPEC 02 §5b)
mg-title = Grupo de máscaras
mg-name = Nombre
mg-mask = Máscaras
mg-hint = Ins añade un grupo, Supr lo quita, F4 edita · un `·` marca los tres incorporados
mg-saved = <{ $name }> está en vigor; Guardar ajustes lo escribe
mg-removed = <{ $name }> ya no está
mg-bad = <{ $name }>: { $problem }
mg-needs-name = un nombre sin espacios, comas ni ángulos
mg-needs-mask = un grupo que no representa nada no coincide con nada
mg-built-in = <{ $name }> es uno de los tres incorporados · F4 le da otras máscaras

sg-title = Grupo de orden
sg-name = Nombre
sg-mask = Máscaras
sg-place = Dónde
sg-place-front = delante de los demás
sg-place-back = detrás de los demás
sg-hint = Ins añade una banda, Supr la quita, F4 la edita, Ctrl+↑↓ la mueve
sg-saved = La banda está en vigor; Guardar configuración la escribe
sg-removed = { $name } ya no está
sg-bad = { $name }: { $problem }
sg-needs-mask = una banda sin máscara no contendría nada
hl-name = Nombre
hl-mask = Máscara o máscaras
hl-kind = Se aplica a
hl-kind-both = archivos y carpetas
hl-kind-file = solo archivos
hl-kind-dir = solo carpetas
hl-size = Tamaño (1M-10M, -4K, 1G-)
hl-age = Edad en días (<=1, >=30, 7-30)
hl-date-field = La edad se refiere a
hl-attrs = Atributos (h r x s b, - para "no debe")
hl-fg = Color (#d7875f, red, file.archive)
hl-bg = Fondo, escrito igual
hl-hint = ↑↓ elegir · Enter/F4 editar · Ins nuevo · Del borrar · Ctrl+↑↓ mover · Esc cerrar
hl-saved = el grupo está activo · Opciones → Guardar ajustes lo escribe en el archivo
hl-removed = { $name } eliminado · Opciones → Guardar ajustes lo escribe en el archivo
hl-bad = { $name }: { $problem }
filter-off = Sin filtro: todos los nombres
filter-hint = ↑↓ elegir · Enter usar · F4 editar · Ins nuevo · Supr quitar · Esc cerrar
filter-new = Nuevo filtro
filter-edit = Filtro
filter-name-hint = con nombre queda en el menú de filtros; sin él se usa una vez
filter-applied = filtro activo: { $name }
filter-cleared = filtro quitado
filter-saved = { $name } está en el panel y en el menú de filtros · Opciones → Guardar los ajustes lo escribe en el archivo
filter-removed = { $name } ya no está · Opciones → Guardar los ajustes lo escribe en el archivo
editor-no-block = no hay nada seleccionado
editor-unique = { $count } líneas repetidas quitadas
editor-no-bracket = no hay ningún paréntesis aquí, o ninguno le corresponde
editor-tabs-expanded = tabulaciones expandidas en { $lines } líneas — al guardar se escribirá
site-protocol = Protocolo
site-protocol-sftp = SFTP (sobre SSH)
site-protocol-ftp = FTP
site-protocol-ftps = FTP con TLS
site-protocol-dav = WebDAV
site-protocol-davs = WebDAV sobre HTTPS
site-auth = Autenticación
site-auth-agent = agente SSH
site-auth-key = archivo de clave
site-auth-ask = pedir la contraseña
site-key-file = Archivo de clave
site-start-dir = Carpeta remota
site-group = Grupo
site-keepalive = Keep-alive (s)
site-compress = Compresión
site-page-session = Sesión
site-page-environment = Entorno
site-page-directories = Carpetas
site-page-sftp = SFTP
site-password-note = No se guarda ninguna contraseña: se pide al conectar.
site-env-eol = Convertir los fines de línea (todavía no)
site-env-utf8 = Forzar los nombres en UTF-8 (todavía no)
site-jump = Host de salto
site-jump-note = Se llega pasando por este host: host, usuario@host o usuario@host:puerto. Vacío significa directo al servidor.
site-sftp-ciphers = Elegir los cifrados (todavía no)
btn-connect = Conectar
site-needs-host = una conexión necesita un servidor
site-needs-key = elige el archivo de clave, u otra autenticación
site-bad-port = «{ $port }» no es un número
site-stored = conexión guardada: { $name } (Shift+F9 para conservarla)
# ------------------------------------------------- panel de estructura (SPEC 05 §1.8)
key-struct = Estruc
key-markdown = MD
key-table = Tabla
key-filter = Filtro
key-decompile = Decomp
key-next = Sig
key-prev = Ant
key-codepages = PágCod
key-goto = IrA
key-to-panel = Panel
key-blame = Blame
viewer-structure = Estructura
viewer-structure-hint = ↑↓ fila · Intro ir a · Tab contenido · F11 cierra
viewer-structure-hint-tab = Tab trae las teclas aquí
viewer-no-structure = nada que descodificar: el panel no conoce este formato
viewer-no-target = esta fila no apunta a ningún lugar del archivo
viewer-form-opened = El formulario, como el texto que Delphi muestra de él. F4 vuelve a los bytes.
viewer-metadata-opened = El texto entero, dispuesto para leerlo. F4 vuelve a los bytes.
viewer-went-to = movido a { $offset }
viewer-in-panel = el panel está en { $name }
viewer-not-in-panel = { $name } no está en este panel
viewer-no-more-files = no hay más archivos en el panel
editor-unsaved-for-viewer = guarda primero: el visor lee el archivo del disco
# ------------------------------------------------- línea de comandos (SPEC 06 §7)
cmd-running = en curso: { $command }
prompt-panelize = Orden cuya salida nombra los archivos
prompt-viewer-filter = Orden cuya salida leer en lugar del archivo
learn-title = Qué teclas envía esta terminal
learn-hint = Pulsa las teclas. Una celda encendida ha llegado; Esc dos veces sale.
learn-last = última: { $key } — { $command }
learn-unbound = sin asignar
recode-title = Reescribir como
recode-what = { $count } archivos
recode-from = Qué son ahora
recode-to = Escribirlos como
recode-auto = deducirlo por archivo
recode-eol = Fin de línea
recode-eol-keep = dejarlos como están
recode-working = reescribiendo { $count } archivos
recode-done = { $changed } escritos, { $same } ya así, { $failed } rechazados
menu-recode = Reescribir como…
panelize-title = { $command }
panelize-done = { $count } archivos, { $dropped } líneas omitidas
panelize-nothing = { $command }: ninguna línea nombra un archivo que exista
cmd-interrupted = interrupción enviada al comando
cmd-done = { $command } terminado
cmd-failed = { $command } salió con { $code }
cmd-busy = ya hay un comando en curso
prompt-apply = Orden a ejecutar en cada uno de los { $count } archivos:
cmd-apply-no-ask = una orden que pide un valor no se puede aplicar archivo por archivo
cmd-apply-progress = { $command }  ({ $done } de { $total })
cmd-apply-stopped = detenido tras { $done } de { $total }: { $command } falló
cmd-not-remote = un comando necesita una carpeta local; este panel está en un servidor
cmd-no-history = el historial de comandos está vacío
cmd-no-previous = no hay sitio al que volver
cmd-no-such-dir = { $path }: no existe esa carpeta
cmd-no-such-path = { $path }: no existe ese archivo o carpeta
cmd-no-selection = no hay nada seleccionado
cmd-condition-false = no ejecutado: { $command }
cmd-screen-cleared = pantalla limpiada
cmd-stack-empty = la pila de carpetas está vacía
cmd-stack-cleared = pila de carpetas vaciada
cmd-pushed = { $count } carpetas en la pila
cmd-environment = entorno
cmd-set = { $name } = { $value }
cmd-not-set = { $name } no está definida
user-screen-title = Salida — { $lines } líneas, desde la línea { $at }
user-screen-empty = (todavía no se ha ejecutado nada)
viewer-no-branch = esta fila no tiene una dirección que seguir
viewer-no-back = no hay nada a lo que volver

# ------------------------------------------------- buscar archivos (SPEC 06 §1)
folders-title = Buscar carpeta
folders-reading = Leyendo las carpetas…
folders-read = { $count } carpetas
folders-capped = { $count } carpetas, hasta donde llega el límite
folders-local-only = la lista de carpetas es la de esta máquina; un servidor tiene su historial y los favoritos
find-title = Buscar archivo
find-mask = Máscara o máscaras
find-size = Tamaño (100M, 1M-1G, -4K)
find-text = Que contienen el texto
find-case = Distinguir mayúsculas
find-words = Solo palabras completas
find-hex = El texto de arriba es un patrón hexadecimal
find-not = Archivos que NO lo contienen
find-scope = Buscar en
find-scope-panel = esta carpeta
find-scope-tree = esta carpeta y las de debajo
find-scope-selection = las carpetas seleccionadas
find-scope-root = desde la raíz de este disco
find-scope-other = el otro panel y lo que hay debajo
find-links = Seguir los enlaces simbólicos
find-dirs = Carpetas también en los resultados
find-archives = Dentro de los archivos comprimidos
find-results = { $count } encontrados · { $files } archivos y { $dirs } carpetas revisados
find-done = { $count } encontrados en { $files } archivos · { $elapsed }
find-none = sin resultados
find-stopped = detenido tras { $files } archivos
find-hint = ↑↓ elegir · Enter ir · F3 ver · F4 editar · F5 al panel · Esc detener
find-hint-done = ↑↓ elegir · Enter ir · F3 ver · F4 editar · F5 al panel · Esc cerrar
find-panelised = { $count } resultados en el panel
find-needs-mask = hace falta una máscara: `*` significa todos los archivos
find-bad-size = `{ $text }` no es un tamaño — { $error }

# ------------------------------------------------- replace across files
replace-title = Sustituir en archivos
replace-needle = Buscar
replace-with = Sustituir por
replace-case = Distinguir mayúsculas
replace-words = Solo palabras completas
replace-pattern = El texto de arriba es una expresión regular
replace-hex = Ambos son bytes hexadecimales
replace-scope = { $count } archivos seleccionados
replace-needs-needle = no hay nada que buscar
replace-confirm-title = Sustituir
replace-confirm-body = Cambiarían { $count } apariciones en { $files } archivos.
replace-nothing = nada que sustituir: ningún archivo cambiaría
replace-done = { $count } apariciones sustituidas en { $files } archivos

# ── archivos comprimidos (SPEC 04 §4) ───────────────────────────────────────────────────
arc-writing-back = { $name } vuelve dentro del archivo comprimido
arc-move-out = para sacar algo de un archivo comprimido, cópialo con F5
arc-add-files-only = de uno en uno solo entran archivos; Shift+F1 mete un directorio entero

rename-title = Renombrar varios
rename-name = Nombre
rename-ext = Extensión
rename-search = Buscar
rename-replace = Reemplazar por
rename-regex = Expresión regular ($1 en el reemplazo)
rename-case = Letras
rename-case-same = como están
rename-case-lower = minúsculas
rename-case-upper = MAYÚSCULAS
rename-case-title = Cada Palabra Con Mayúscula
rename-case-sentence = Solo la primera letra
rename-counter = Contador: inicio, paso, cifras
rename-fields = [N] nombre  [E] extensión  [N1-3] parte  [C] contador  [d] [t] fecha, hora  [P] carpeta
rename-not-in-archive = varios archivos se renombran en una carpeta, no dentro de un archivo comprimido: allí F6 renombra una entrada
rename-bad-counter = el contador son tres números enteros, inicio, paso, cifras: { $counter } no lo es
rename-cannot = no se puede renombrar: { $why }
rename-problem-empty = no queda ningún nombre
rename-problem-invalid = no es un nombre que un archivo pueda tener
rename-problem-twice = otro archivo también toma este nombre
rename-problem-exists = ya hay un archivo con este nombre
rename-more = … y { $count } más
rename-blocked =
    { $count ->
        [one] un archivo no puede tomar su nuevo nombre: no se renombra nada hasta que pueda
       *[other] { $count } archivos no pueden tomar su nuevo nombre: no se renombra nada hasta que puedan
    }
rename-summary = cambian { $count } de { $total }
rename-nothing = ningún nombre cambia
rename-nothing-to-undo = no hay ningún renombrado múltiple que deshacer
rename-done =
    { $count ->
        [one] un archivo renombrado · se deshace desde el menú Archivos
       *[other] { $count } archivos renombrados · se deshacen desde el menú Archivos
    }
rename-undone =
    { $count ->
        [one] un nombre devuelto
       *[other] { $count } nombres devueltos
    }
pack-title = Añadir al archivo
pack-name = Archivo
pack-format = Formato
pack-level = Compresión
pack-solid = Bloques sólidos (7z: más pequeño, más lento para un archivo)
pack-delete-after = Eliminar los archivos tras archivar
pack-level-normal = normal
pack-level-fast = rápida
pack-level-best = máxima
pack-scope = { $count } archivos seleccionados
pack-needs-name = el archivo necesita un nombre
pack-format-read-only = { $format } aquí se lee, no se escribe: elige otro formato u otro nombre
pack-single-one-file = un { $format } contiene un solo archivo: elige uno, o usa { $tar } para una carpeta o varios archivos
unpack-title = Extraer del archivo
unpack-to = Extraer en
unpack-scope = de { $name }
unpack-not-an-archive = no es un archivo que sepamos leer

# ── los paneles especiales (SPEC 02 §7) ─────────────────────────────────────────────────
panel-quick-view = Vista rápida
panel-info = Información
panel-tree = Árbol
compare-done = { $left } a la izquierda, { $right } a la derecha son distintos
cmp-subdirs = Entrar en las subcarpetas
cmp-depth = Hasta qué profundidad (0 = hasta el fondo)
cmp-trees-title = Lo que difiere
cmp-trees-working = comparando los árboles: { $seen } archivos, { $read } leídos
cmp-trees-done = { $differ } difieren, { $left } solo a la izquierda, { $right } solo a la derecha, de { $pairs } parejas
cmp-trees-same = los dos árboles son iguales ({ $pairs } parejas, { $same } comprobadas)
menu-duplicates = Buscar duplicados
dup-title = Archivos duplicados
dup-working = buscando duplicados: { $seen } archivos, { $hashed } leídos
dup-done = { $groups } grupos de copias, { $files } archivos, { $wasted } desperdiciados
dup-none = aquí nada es copia de otra cosa ({ $seen } archivos)
dup-stopped = detenido tras { $seen } archivos
dup-partial = detenido antes del final
cmp-title = Comparar
cmp-case = Distinguir mayúsculas y minúsculas en los nombres
cmp-size = Un tamaño distinto es una diferencia
cmp-time = Una fecha distinta es una diferencia
cmp-seconds = Fechas con esta diferencia en segundos son el mismo momento
cmp-contents = Leer los bytes
cmp-when-time = Leerlos solo donde las fechas difieren
cmp-fragment = Leer solo los primeros KiB (0 = todo)
cmp-ignore-eol = Los fines de línea por sí solos no son una diferencia
cmp-selected = Solo lo seleccionado
cmp-stop = Parar en la primera diferencia
cmp-newer = Seleccionar solo la copia más reciente
menu-compare-setup = Comparar con opciones
compare-contents-eol-ignored = { $count } difieren solo en los fines de línea, ignorados
compare-contents-trusted = { $count } tomados como iguales sin leerlos
compare-contents-working = Comparando contenidos: { $done } de { $total }
compare-contents-done = { $differ } de { $pairs } parejas son distintas
compare-contents-same = las { $pairs } parejas son idénticas
compare-contents-none = ningún archivo tiene el mismo nombre en ambos lados
compare-contents-local = comparar por contenido necesita ambos paneles en un sistema de archivos local
compare-contents-eol = { $count } solo en los fines de línea
compare-contents-only = { $count } solo en un lado
compare-contents-unread = { $count } no se han podido leer
compare-contents-stopped = comparación detenida
quick-view-directory = { $name } es un directorio — F3 lo mide
quick-view-not-here = los bytes no están aquí: F3 los extrae primero
info-path = Ruta
info-kind = Tipo
info-kind-local = un directorio en esta máquina
info-kind-remote = en { $host }
info-kind-archive = dentro de { $name }
info-kind-list = una lista de archivos
info-device = Disco
info-folder = Carpeta
info-device-value = { $total }, { $free } libres, clústeres de { $cluster }
info-slack = Desperdicio
info-filesystem = Sistema de archivos
info-mount = Montado
info-git = Repositorio
info-git-clean = como en el índice
info-git-differing = { $count } de { $tracked } difieren del índice
info-git-uncounted = sin contar
info-git-merge = fusión en curso
info-git-rebase = rebase en curso
info-git-cherry-pick = cherry-pick en curso
info-git-revert = revert en curso
info-git-bisect = bisect en curso
info-memory = Memoria
info-memory-value = { $free } libres de { $total }
info-memory-total = { $total }
info-contents = Contiene
info-contents-value = { $files } archivos, { $dirs } directorios, { $bytes }
info-selected = Seleccionados
info-selected-value = { $count } archivos, { $bytes }
info-cursor = Cursor
info-size = Tamaño
info-modified = Modificado
info-description = Descripción
key-mark = Marca
key-asm = Asm
key-block = Bloque
key-screens = Pantallas
key-site-edit = EditSes
key-site-new = NuevSes
key-site-forget = BorrSes
key-ai-edit = Editar
key-ai-new = Nuevo
key-ai-remove = Quitar
key-ai-forget = Quitar
key-ai-threads = Historial
hex-needs-a-file = este búfer no tiene un archivo del que leer bytes
hex-save-first = guarda primero: los dos editores no pueden pasarse los cambios sin guardar

hex-block-title = Bloque
hex-op-write = Escribir los { $size } en un archivo
hex-op-read = Leer un archivo en { $at }
hex-op-insert = Insertar bytes cero en { $at }
hex-op-copy = Copiar bytes de otro sitio a { $at }
hex-op-move = Mover bytes de otro sitio a { $at }
hex-op-fill = Rellenar los { $size } con un byte
hex-op-xor = O-exclusivo de los { $size } con una clave
hex-op-delete = Eliminar los { $size } — { $was } pasa a { $becomes }
hex-write-title = Escribir el bloque en
hex-read-title = Qué archivo leer
hex-fill-title = Rellenar con qué byte (hex)
hex-xor-title = O-exclusivo con qué clave (hex)
hex-insert-title = Cuántos bytes cero
hex-copy-title = Copiar desde dónde — posición y longitud (hex)
hex-move-title = Mover desde dónde — posición y longitud (hex)
hex-not-hex = eso no son dígitos hexadecimales
hex-not-a-count = eso no es un número de bytes
hex-not-a-range = eso no es una posición con una longitud en hexadecimal
hex-range-outside = ese intervalo está fuera del archivo ({ $size })
hex-move-into-itself = un bloque no se puede mover dentro de sí mismo
hex-filled = { $size } rellenados
hex-xored = o-exclusivo sobre { $size } — esto no es cifrado
hex-written = { $size } escritos en { $name }
hex-read-done = { $size } leídos de { $name }
hex-inserted = insertados { $size } de ceros
hex-deleted = eliminados { $size }
hex-device = { $name }: un dispositivo de { $size }, abierto solo para lectura
asm-title = Ensamblar en { $at } ({ $arch })
asm-no-arch = { $arch } no es una arquitectura que sepamos ensamblar
asm-written = { $text } — { $bytes }
asm-written-longer = { $text } — { $bytes } ({ $grew } bytes más)
screens-title = Pantallas
screens-panels = Paneles
quit-title = Salir de FinalCommander
quit-body = ¿Salir?
screens-quit-title = Cambios sin guardar
screens-quit-body = Estas pantallas tienen cambios sin escribir:
macro-recording = Grabando una macro · Ctrl+. la detiene
macro-empty = no se grabó nada
macro-dropped = grabación descartada
macro-key-title = Qué tecla
macro-key-body = { $steps } pasos grabados. Pulsa la tecla a la que responderá esta macro, o Esc para descartarla.
macro-describe = Para qué sirve la macro en { $key }
macro-saved = macro en { $key } guardada, { $steps } pasos
macro-none = ninguna macro grabada · Ctrl+. graba una
macro-list-title = Macros
macro-play-title = Cómo debe ejecutarse
macro-play-once = Una vez, desde donde está el cursor
macro-play-each = Una vez por cada archivo seleccionado
macro-sure-title = Ejecutar esta macro
macro-sure-body = La macro en { $key } copia, mueve, borra o sobrescribe. ¿Ejecutarla?
macro-too-deep = una macro no puede llamarse a sí misma sin fin
plugin-list-title = Comandos de los complementos
plugin-none-title = No hay complementos instalados
plugin-none-open = Abrir la carpeta de complementos ({ $dir })
plugin-none-help = Cómo funcionan los complementos y dónde encontrarlos
plugin-help-heading = Plugins
plugin-why-off = desactivado en Opciones → Configuración de complementos
plugin-why-panel = solo en los paneles
plugin-why-viewer = solo en el visor
plugin-why-editor = solo en el editor
plugin-why-local = necesita una carpeta en los discos de esta máquina
plugin-why-files = necesita un archivo bajo el cursor o seleccionado
plugin-why-masks = solo para { $masks }
plugin-why-bad-mask = su máscara es incorrecta: { $error }
plugin-why-here = necesita { $names } en esta carpeta
plugin-why-inside = necesita { $names } aquí o en una carpeta superior
plugin-cannot = { $label }: { $why }
plugin-running = { $label }…
plugin-on-selected = sobre { $count } archivo(s) seleccionado(s)
plugin-failed = { $id }: { $why }
plugin-unknown-command = un complemento pidió «{ $id }», que no es un comando
hex-copied = { $size } copiados desde { $from }
hex-moved = { $size } movidos desde { $from }
menu-calculator = Calculadora
menu-terminal = T&erminal
calc-title = Calculadora
calc-decimal = decimal
calc-hex = hex
calc-octal = octal
calc-binary = binario
calc-characters = caracteres

# The archive commands (Shift+F3) and the two parts of the order that are not the key.
arc-commands-title = Comandos de archivo
arc-password = Contraseña para { $name }
arc-op-test = Probar { $name }
arc-op-extract-here = Extraer { $name } aquí
arc-op-extract-to = Extraer { $name } en…
arc-op-delete = Eliminar { $name }
arc-op-add = Añadir archivos a un archivo…
job-test = Probando archivo
msg-dirs-first-on = carpetas primero
msg-dirs-first-off = carpetas junto a los archivos
msg-selected-first-on = archivos seleccionados primero
msg-selected-first-off = archivos seleccionados en orden

# The change-drive dialog (Alt+F1, Alt+F2): the type column, and its footer.
drive-fixed = Disco
drive-removable = Extraíble
drive-optical = Óptico
drive-network = Red
drive-memory = Memoria
drive-home = Personal
drive-connection = Conexión
drive-objects = Almacén de objetos
drive-netbox = NetBox: sesiones guardadas
drive-processes = Lista de procesos
drive-temp = Panel temporal
temp-added = { $count } en el panel temporal
proc-none = aquí no se puede leer la lista de procesos
drive-programs = Programas instalados
drive-services = Servicios
svc-none = esta máquina no tiene un gestor de servicios al que preguntar
temp-not-a-file = las filas aquí no son archivos: no hay nada que borrar
svc-card = Servicio
svc-card-id = Nombre
svc-card-name = Se muestra como
svc-card-state = Estado
svc-card-description = Descripción
svc-card-pid = Proceso
svc-state-running = en marcha
svc-state-stopped = parado
svc-state-failed = fallido
svc-state-other = otro
key-service-start = Iniciar
key-service-restart = Reiniciar
key-service-stop = Parar
svc-ask-stop = ¿Parar { $name }?
svc-ask-restart = ¿Reiniciar { $name }?
btn-service-stop = Parar
btn-service-restart = Reiniciar
svc-working = { $name }: preguntando al sistema…
svc-started = { $name } iniciado
svc-stopped = { $name } parado
svc-restarted = { $name } reiniciado
svc-needs-root = { $name }: esto necesita root — el programa no lo pedirá por ti
svc-failed = { $name }: { $error }
prog-none = aquí nada dice qué está instalado
prog-card = Programa instalado
prog-no-uninstall = nada dice cómo quitar { $name }
prog-uninstall-ready = la orden que quita { $name } está en la línea de órdenes, sin ejecutar
prog-card-name = Nombre
prog-card-version = Versión
prog-card-publisher = Editor
prog-card-size = Espacio en disco
prog-card-installed = Instalado
prog-card-location = Carpeta
prog-card-uninstall = Se quita con
drive-sockets = Lista de sockets
sock-none = aquí no hay nada escuchando, o la tabla no se puede leer
sock-no-process = nada dice qué proceso tiene este socket
proc-card = Proceso
proc-gone = { $name } ya no se está ejecutando
proc-card-name = Nombre
proc-card-pid = PID
proc-card-parent = Padre
proc-card-user = Usuario
proc-card-state = Estado
proc-card-threads = Hilos
proc-card-memory = Memoria
proc-card-mapped = Asignada
proc-card-bits = Bits
proc-card-started = Iniciado
proc-card-exe = Ejecutable
proc-card-cwd = Carpeta
proc-card-command = Línea de órdenes
proc-card-group = Grupo
proc-card-sid = SID
proc-card-priority = Prioridad
proc-card-session = Sesión
proc-card-uptime = En ejecución desde hace
proc-card-cpu-user = Tiempo de CPU, propio
proc-card-cpu-kernel = Tiempo de CPU, en el núcleo
proc-card-handles = Descriptores abiertos
proc-card-faults = Fallos de página
proc-card-private = Memoria privada
proc-card-swap = En intercambio
proc-card-io-read = Leídos
proc-card-io-write = Escritos
proc-card-memory-peak = Pico del conjunto de trabajo
proc-card-private-resident = Conjunto de trabajo privado
proc-card-swap-peak = Pico del archivo de paginación
proc-card-virtual = Bytes virtuales
proc-card-virtual-peak = Pico de bytes virtuales
proc-card-paged-pool = Bloque paginado
proc-card-nonpaged-pool = Bloque no paginado
proc-card-io-other = Otra E/S
proc-card-io-read-ops = Operaciones de lectura
proc-card-io-write-ops = Operaciones de escritura
proc-card-io-other-ops = Otras operaciones
proc-kill-title = Terminar proceso
proc-kill-ask = ¿Terminar { $name } [{ $pid }]? Lo que no haya guardado se pierde.
proc-killing = Terminando { $name }…
proc-killed = { $name } terminado
proc-kill-failed = { $name } no se puede terminar: { $why }
proc-card-env = Entorno
proc-card-modules = Módulos cargados
proc-card-version = Versión
proc-card-description = Descripción
drives-hint = ↑↓ elegir · 0-9 paneles · Ctrl+1..9 columnas · F9 todas · Ctrl+R releer
drives-title = Cambio de unidad: qué mostrar
drives-col-type = el tipo
drives-col-device = el dispositivo
drives-col-label = la etiqueta
drives-col-fs = el sistema de archivos
drives-col-size = los tamaños
drives-col-removable = las unidades extraíbles
drives-col-panels = los paneles que no son discos
drives-col-optical = las unidades ópticas
drives-col-network = los recursos de red
drives-shown = mostrando { $what }
drives-hidden = ocultando { $what }
drives-reread = unidades leídas de nuevo
bookmarks-hint = Intro ir · Ins guardar · Supr quitar · F4 nombrar · Esc cerrar

# The search dialog (F7), which asks the whole question in one place.
search-title = Buscar
search-for = Buscar
search-mode = Como
search-mode-text = texto
search-mode-hex = hex
search-mode-regex = expresión regular
search-case = Distinguir mayúsculas
search-words = Solo palabras completas
btn-find-next = Buscar siguiente
btn-find-prev = Buscar anterior
help-chapter-about = Qué es esto
help-chapter-panels = Los paneles
help-chapter-files = Archivos: copiar, mover, buscar, archivos comprimidos
help-chapter-viewer = El visor
help-chapter-editor = El editor, el panel hexadecimal y el ensamblador
help-chapter-network = Servidores
help-chapter-config = Configuración, teclas y colores
help-chapter-keys = Las teclas, tal como están asignadas ahora
help-chapter-commands = Todos los comandos y sus teclas
help-chapter-options = Todos los ajustes y su valor
help-here-panels = Los paneles
help-here-viewer = El visor
help-here-editor = El editor
help-here-hex = El editor hexadecimal
help-here-dialog = Este diálogo
help-in-english = Este capítulo todavía no está traducido a tu idioma, así que sigue en inglés. Las teclas y los ajustes que menciona son los mismos en todos los idiomas.
help-about-byline = FinalCommander { $version } — por Matteo Baccan
key-ok = OK
key-cancel = Cancel
key-dialog-move = Mover
key-tree = Árbol
key-config = Config
browse-local-only = un archivo se elige en esta máquina; una ruta en un servidor se escribe, y Tab la completa
key-browse = Examin
table-prompt-filter = Filas que contienen
table-sorted = ordenadas por { $column }
table-sorted-back = ordenadas por { $column }, al revés
table-unsorted = de nuevo en el orden del archivo
table-filtered = quedan { $rows } filas
table-unfiltered = todas las filas otra vez
table-cut-sorted = de las primeras { $rows } filas, que es todo lo leído
table-too-many = demasiados registros para ordenarlos o filtrarlos aquí
table-copied =
    { $rows ->
        [one] una fila en el portapapeles
       *[other] { $rows } filas en el portapapeles
    }
table-headed = la primera línea es la cabecera
table-not-headed = la primera línea son datos: las columnas se llaman por su número
table-named-fields = este formato nombra sus propios campos: no hay cabecera que adivinar

# Splitting and combining (SPEC 03 §9)
split-title = Dividir archivo
split-target = En
split-size = Cada trozo
split-size-hint = 1440K, 700M, 4G — el último trozo es lo que queda
split-count = O en
split-count-hint = partes — un número aquí manda sobre el tamaño
split-bad-count = { $count } no es un número de partes: dos o más
split-local-file = Dividir y unir funcionan con un archivo de esta máquina
split-bad-size = «{ $size }» no es un tamaño: escríbalo como 700M o 4G
combine-not-a-piece = No es un trozo: los trozos de un archivo dividido terminan en .001, .002, … o traen un .crc
pieces-working = Dividiendo o uniendo: { $done } de { $total }
pieces-cancelled = Detenido, y lo escrito se ha retirado
split-done = { $parts } trozos escritos, con { $list } al lado
combine-done-checked = { $name } unido de { $parts } trozos, { $bytes }, y coincide con su .crc
combine-done-unchecked = { $name } unido de { $parts } trozos, { $bytes } — no había un .crc con que comprobarlo
pieces-failed = No hecho: { $error }

# The branch view (SPEC 02 §3d)
branch-reading = Leyendo todos los archivos bajo { $path }…
branch-title = Rama: { $path }
branch-done = { $count } archivos, de todas las carpetas de debajo · Ctrl+Alt+B vuelve
branch-truncated = { $count } archivos, y la lectura se detuvo ahí: la rama es mayor de lo que una lista puede servir
branch-not-here = La vista de rama es de una carpeta, y este panel es una lista

## The release check (SPEC 11 §5).
menu-check-update = &Buscar actualizaciones
opt-update-check = Buscar una versión nueva una vez al día
btn-open-page = Abrir la página
update-checking = Preguntando a GitHub por la última versión…
update-title = Actualizaciones
update-available = Ha salido FinalCommander { $version } (esta es la { $current }).
update-available-status = Ha salido FinalCommander { $version }: Opciones → Buscar actualizaciones
update-current = Esta es la última versión, la { $version }.
update-failed = No se pudo preguntar por la última versión: { $why }

## Duplicate files (SPEC 06 §13).
menu-dupes = Buscar archivos duplicados
dupes-title = Archivos duplicados
dupes-recursive = Buscar en subcarpetas
dupes-min-size = Archivo más pequeño
dupes-min-size-hint = Un tamaño como lo escriben los filtros: 1, 100K, 5M. Los archivos vacíos nunca se comparan.
dupes-select = Seleccionar todas las copias menos la más antigua
btn-search = Buscar
dupes-not-here = La búsqueda de duplicados parte de una carpeta o de lo seleccionado.
dupes-bad-size = No es un tamaño: { $size }
dupes-looking = Buscando archivos duplicados…
dupes-progress = Duplicados: pasada { $pass } de 3, { $done }/{ $total }
dupes-none = Ningún duplicado entre { $count } archivos.
dupes-panel = Duplicados: { $groups } grupos
dupes-done = { $groups } grupos de duplicados, { $size } en las copias.
dupes-truncated = Detenida antes: { $groups } grupos por ahora, { $size } en las copias.

## SQLite queries (SPEC 04 §4b).
sql-empty = Escribe una consulta tras sql:, por ejemplo sql: SELECT * FROM t
sql-no-database = sql: necesita una base SQLite: entra en ella o pon el cursor sobre una.
sql-running = Ejecutando la consulta…
sql-failed = La consulta falló: { $why }

## Asking a language model (SPEC 06 §14).
ai-none = No hay ningún servicio de IA: añade un bloque [[ai]] a fcmd.toml (name, url, model, api, token_env).
ai-title = Preguntar a una IA
ai-no-thread = no hay respuesta que continuar: pregunta antes con `ai:`, o pon el cursor sobre una en la carpeta de respuestas
ai-nothing-to-explain = nada que explicar aquí: abre un archivo en el visor o en el editor
ai-explain-question = ¿Qué es esto, de { $name }? Explícalo brevemente.
ai-tokens = { $in } tokens de entrada, { $out } de salida
ai-none-panel = Aún no hay servicios de IA — F7 añade uno
ai-service-removed = { $name } quitado
ai-probing = Conectando con { $name } en { $host }…
ai-probe-ok = { $host } respondió HTTP { $status } en { $ms } ms, { $models } modelos — escribe una pregunta y pulsa Intro
ai-probe-no-model = (el modelo { $model } no está entre ellos)
ai-row-context = contexto
ai-row-conversation = conversación
ai-session-title = IA: { $name } · { $model } — { $files } archivos en contexto
ai-context-added = { $count ->
        [one] Un archivo
       *[other] { $count } archivos
    } en el contexto
ai-context-nothing = Nada que añadir: al contexto solo van archivos de este disco
ai-command-ready = la orden está en la línea — léela antes de pulsar Intro
ai-commit-question = Escribe un mensaje de commit para los cambios preparados.
ai-staged-diff = el diff preparado
ai-send-diff = El diff preparado va a { $host }
ai-send-files = { $count } archivos, { $size }, se enviarán a { $host } con la pregunta.
ai-asking = Preguntando a { $name }…
ai-busy = { $name } está pensando… { $secs } s · Esc detiene
ai-conversation-new = Una conversación nueva: la próxima pregunta la empieza.
ai-live-empty = Escribe una pregunta en la línea de órdenes: va a { $name }, y la conversación se ve aquí.
ai-failed = La IA no respondió: { $why }
ai-stopped = detenida: la pregunta a { $name } se ha descartado y no se ha escrito nada
ai-context-title = Archivos en el contexto
ai-context-all = Todos
ai-context-dropped = { $count ->
        [one] un archivo fuera del contexto
       *[other] { $count } archivos fuera del contexto
    }
ai-context-hint = Intro o Supr: sacarlo del contexto — el archivo sigue en el disco · F3: lo que se envía · Esc
ai-threads-title = Conversaciones
ai-threads-hint = Intro: seguir con esta conversación · Esc
ai-thread-resumed = se sigue con { $name }
ai-next-estimate = siguiente ≈ { $tokens }
ai-next-over = siguiente ≈ { $tokens }, más de lo que cabe en el modelo
ai-send-estimate = Unos { $tokens } tokens en total, con la conversación hasta ahora.
ai-send-estimate-of = Unos { $tokens } tokens en total, con la conversación hasta ahora, de los { $max } que caben en el modelo.
ai-send-over = Unos { $tokens } tokens en total: más de los { $max } que caben en el modelo, y el principio puede quedar cortado. F8 saca un archivo.
ai-send-question = La pregunta va a { $host }.
btn-send = Enviar

## Git (SPEC 06 §15).
menu-git = Git
git-title = Git
git-status = Lo que ha cambiado
git-log = El registro
git-blame = Blame de { $name }
git-branches = Ramas
git-local-only = git trabaja con una carpeta de este disco.
git-running = Preguntando a git…
git-failed = git: { $why }
git-clean = No ha cambiado nada.
git-status-title = Cambiados: { $staged } preparados, { $unstaged } no
git-side-conflict = conflicto
git-side-staged = preparado
git-side-changed = sin preparar
git-side-untracked = sin seguimiento
git-kind-modified = modificado
git-kind-added = nuevo
git-kind-deleted = borrado
git-kind-renamed = renombrado
git-kind-copied = copiado
git-kind-type = tipo cambiado
git-kind-conflict = por resolver
git-as-tree = Mostrar los cambios como árbol
git-as-list = Mostrar los cambios como lista
git-version-head = último commit
git-version-index = preparado
git-version-disk = en disco
git-discard-title = Descartar cambios
git-discard-body = ¿Descartar los cambios { $count ->
        [one] de un archivo
       *[other] de { $count } archivos
    }?
git-discard-worktree = como en el índice:
git-discard-both = como en el último commit:
git-discard-added = fuera del índice, sigue en disco:
git-discard-trash = a la papelera:
git-discard-warning = Lo que vuelve al índice o al último commit no se puede deshacer.
git-hint = Intro: elegir · Esc: cerrar

## The editor's character list and word completion.
charmap-title = Caracteres
charmap-hint = Escribe para filtrar (un carácter o su código, U+00E9) · Intro: insertar · Esc: volver
editor-no-completion = Ninguna otra palabra del archivo empieza así.

## A stored session's address, copied (NetBox's Generate URL).
net-url-copied = Copiado { $url }

## The system log (SPEC 06 §16).
menu-syslog = Registro del sistema
syslog-reading = Leyendo el registro del sistema…

## Deleting lines by pattern (FAR #10).
editor-prompt-delete-matching = Quitar las líneas que coinciden (! delante: conservar solo esas)
editor-lines-deleted = Líneas quitadas: { $count }

## The copy dialog's size policy (FAR #493).
copy-conflict-size = sobrescribir si el tamaño es distinto

# Copy dialog: what an error does (FAR's issue #597)
copy-on-error = Si uno no se puede copiar
copy-error-ask = Preguntar
copy-error-skip = Omitirlo y seguir
copy-error-stop = Detener la copia

# Attributes dialog: the times copied from one file to another (FAR's issue #241)
attr-times-copied = Horas copiadas: Ctrl+Mayús+V las pone en el diálogo de otro archivo
attr-times-none = Nada copiado aún: primero Ctrl+Mayús+C en el diálogo de un archivo
attr-times-pasted = Horas pegadas: Aceptar las aplica

# Editor: replacements kept by name and recent ones (FAR's issue #282)
replace-tpl-title = Reemplazos
replace-tpl-hint = Intro lo ejecuta · Ins guarda uno reciente con nombre · Supr olvida uno guardado · Esc
replace-tpl-name = Guardar este reemplazo como
replace-tpl-kept = Guardado como {$name}
replace-tpl-empty = Aún no hay reemplazos: primero Ctrl+F7

# Panel settings: a view and a sort per folder (FAR's issue #389)
opt-remember-view = Cada carpeta conserva su vista y su orden

# Session editor: a cap on a copy's speed to or from the site
site-max-kbps = Límite de velocidad, KB/s (vacío: ninguno)

# Session editor: IMAP, read only
site-protocol-imap = IMAP (correo, solo lectura)
site-protocol-imaps = IMAPS (correo sobre TLS, solo lectura)

# The AI services as a list, like the sessions
ai-services-title = Servicios de IA
ai-services-hint = Intro: el que pregunta ai: · Ins nuevo · F4 cambiar · Supr quitar · Esc
ai-service-title = Servicio de IA
ai-field-name = Nombre
ai-field-api = Protocolo
ai-field-url = Dirección
ai-field-model = Modelo
ai-field-token-env = Clave en la variable
ai-field-token-note = Vacío: la clave se pide una vez por sesión y nunca se guarda.
ai-field-system = Lo que se le dice antes
ai-service-incomplete = Un servicio necesita nombre y dirección
ai-service-default = ai: ahora pregunta a {$name}
ai-key-prompt = La clave de {$name} (se guarda hasta salir)
ai-api-openai = OpenAI y compatibles
ai-api-anthropic = Anthropic
ai-api-ollama = Ollama (en esta máquina)
menu-ai = Servicios de IA

# The secrets file
vault-password = La contraseña del archivo de secretos de fcmd
vault-new = Guardar esta clave en el archivo cifrado de secretos de fcmd: elige su contraseña (vacío: solo esta sesión)
vault-refused = El archivo de secretos no se abrió: {$why}

# Programas externos
drive-tools = Programas externos
drive-ai = IA
drive-git = Git
tools-found = encontrado
tools-missing = no instalado
tools-summary = { $found } de { $total } programas externos encontrados; F3 dice para qué sirve cada uno, F4 instala uno que falta
tools-card = Programa externo
tools-card-program = Programa
tools-card-where = Dónde
tools-card-version = Versión
tools-card-for = Se usa para
tools-card-install = Para instalarlo
tools-card-why = Por qué fcmd lo ejecuta en vez de hacerlo él mismo: F1, «Programs from elsewhere».
tools-install-any = instala el paquete que trae { $name } y comprueba que esté en el PATH
tools-already = { $name } ya está
tools-for-archives = archivos comprimidos: { $what }
tools-for-decompile = Shift+F6 en el visor sobre { $what }
tools-for-git = el panel de Git (Ctrl+Alt+G) y la rama en el prompt
tools-for-video = los fotogramas de un vídeo en el visor y en la vista rápida (F3, Ctrl+Q)
tools-for-open = abrir un enlace en el navegador
tools-for-syslog = Órdenes → Registro del sistema
tools-for-services = el panel de servicios
tools-card-site = Sitio web
tools-no-package = Ningún gestor de paquetes de esta máquina tiene { $name } que sepamos. ¿Abrir su página de descarga?
tools-install-ask = Instalar { $name } ejecutando, en un terminal:
tools-install-how = En el terminal sudo puede pedir tu contraseña. Ctrl+Alt+E vuelve a los paneles y Ctrl+R en este panel busca de nuevo.
tools-install-else = Si falla: { $url }
tools-install-failed = fcmd: { $name } no se instaló. Descárgalo de { $url }
tools-installing = instalando { $name } en un terminal: Ctrl+Alt+E vuelve a los paneles, Ctrl+R allí busca de nuevo
btn-install = Instalar

# Configuración de plugins
plugins-config-title = Plugins
plugins-config-hint = Enter: activar o desactivar · F3 qué es y qué trae · Esc
plugins-broken = no se cargó — F3 dice por qué
plugins-cannot-toggle = un plugin que no se cargó no se puede activar: F3 dice qué le pasa
plugins-on = { $id } está activado: sus órdenes ya están en F11
plugins-on-restart = { $id } está activado: sus órdenes ya están en F11, el resto desde el próximo inicio
plugins-off = { $id } está desactivado: sus órdenes han salido de F11
plugins-off-restart = { $id } está desactivado: sus órdenes han salido de F11, el resto se va en el próximo inicio
plugins-state-on = activado
plugins-state-off = desactivado
plugins-card-id = Id
plugins-card-name = Nombre
plugins-card-version = Versión
plugins-card-author = Autor
plugins-card-about = Descripción
plugins-card-dir = Carpeta
plugins-card-state = Estado
plugins-card-brings = Trae
plugins-card-commands = Órdenes (F11):
plugins-card-notes = Dicho al cargar:
plugins-none = no se encontraron plugins · van en { $dir }, una carpeta cada uno con un plugin.toml

# The git panel
git-file-log = El log de { $name }
git-commit = Hacer commit de lo preparado…
git-pull = Pull
git-push = Push
git-pushing = Haciendo push…
git-pulling = Haciendo pull…
git-commit-prompt = El mensaje del commit
git-commit-empty = Un commit necesita un mensaje.
git-already-on = { $branch } es la rama en la que estás.
git-switch-ask = ¿Cambiar a { $branch }? Los archivos en que las dos ramas difieren cambian en el disco.
git-done = git: { $said }
git-branches-title = Ramas: { $count }
git-log-title = Log: { $count }
git-log-of-title = Log de { $of }: { $count }
btn-switch = Cambiar
key-git = Git
key-git-diff = Diff
key-git-stage = Stage
key-git-unstage = Unstage
key-git-discard = Descartar
key-git-commit = Commit
key-git-show = Ver
key-git-log = Log
key-git-changes = Cambios
key-git-pull = Pull
key-git-push = Push
key-git-branches = Ramas

undo-title = Deshacer
btn-undo = Deshacer
menu-undo = Deshacer la última operación
menu-strip-meta = Quitar los metadatos de las imágenes…
undo-nothing = nada que deshacer: ninguna copia, movimiento, borrado a la papelera ni carpeta nueva desde que empezó el programa
undo-copy =
    { $count ->
        [one] Mandar a la papelera el archivo que hizo la copia:
       *[other] Mandar a la papelera los { $count } archivos que hizo la copia:
    }
undo-move =
    { $count ->
        [one] Devolver el archivo a su sitio:
       *[other] Devolver los { $count } archivos a su sitio:
    }
undo-trash =
    { $count ->
        [one] Recuperar el archivo de la papelera:
       *[other] Recuperar los { $count } archivos de la papelera:
    }
undo-mkdir = Quitar la carpeta nueva, si sigue vacía:
strip-title = { $count ->
        [one] Metadatos de una imagen
       *[other] Metadatos de { $count } imágenes
    }
strip-all = Todo — texto, EXIF, XMP, IPTC, comentarios, C2PA
strip-ai = Solo lo que escribió un modelo de imágenes — prompt, workflow, ajustes
strip-camera = Todo menos el EXIF de la cámara y el copyright — sin GPS
strip-hint = Intro: quitarlos; la imagen no se recodifica, Ctrl+Alt+Z la deja como estaba · Esc
strip-nothing = ningún PNG, WebP o JPEG de este disco seleccionado o bajo el cursor
strip-done = { $count ->
        [one] una imagen
       *[other] { $count } imágenes
    } sin metadatos ({ $blocks } bloques quitados), { $clean } no tenían · Ctrl+Alt+Z las deja como estaban
undo-restore = { $count ->
        [one] Devolver la imagen con sus metadatos:
       *[other] Devolver las { $count } imágenes con sus metadatos:
    }
undo-done =
    { $count ->
        [one] un nombre de vuelta en su sitio
       *[other] { $count } nombres de vuelta en su sitio
    }
slow-drives = la lista de unidades tardó { $secs } s
slow-drives-late = la lista de unidades esperó { $secs } s: { $drives } no respondieron a tiempo y se muestran sin detalles — ¿una unidad de red sin servidor?
slow-listing = leer { $what } tardó { $secs } s
# editor search and undo (0.41)
btn-find-all = Todas las coincidencias
btn-replace-all = Sustituir todo
btn-replace-one = Sustituir el siguiente
editor-replace-title = Sustituir
editor-replace-style = Conservar el estilo (foo → bar, Foo → Bar, FOO → BAR)
editor-replace-groups = En una expresión regular $1 y { "${name}" } son lo que capturó un grupo, $$ un dólar
editor-matches-title = Líneas con `{ $query }`: { $count }
editor-matches-hint = Escribe para filtrar · Intro: ir a la línea · Esc: volver


# sorting and extended attributes (0.41)
opt-sort-case = Mayúsculas y minúsculas al ordenar
opt-sort-case-auto = como este sistema
opt-sort-case-yes = distinguirlas
opt-sort-case-no = un solo alfabeto
copy-keep-xattrs = Conservar los atributos extendidos

# editor conversions, disk diff and guides (0.41)
editor-convert-title = Escribir el archivo como
editor-convert-hint = Escriba para filtrar · Intro: convierte (F2 lo escribe, Ctrl+Z lo deshace) · Esc: volver
editor-convert-eol = Fin de línea: { $eol }
editor-convert-bom-on = Marca de orden de bytes (BOM): se escribe
editor-convert-bom-off = Marca de orden de bytes (BOM): no se escribe
editor-convert-page = Página de códigos: { $page }
editor-converted-eol = { $name }: las líneas terminarán con { $what } al guardar
editor-converted-bom-on = { $name }: al guardar se escribirá la marca de orden de bytes
editor-converted-bom-off = { $name }: al guardar no se escribirá la marca de orden de bytes
editor-converted-page = { $name }: al guardar se escribirá en { $what }
editor-convert-same = El archivo ya se escribe así
editor-convert-no-bom = { $page } no tiene marca de orden de bytes: solo UTF-8 y UTF-16 la tienen
editor-convert-lossy = { $page } no puede contener todos los caracteres de { $name }: no se cambió nada
btn-reload = Recargar
btn-what-changed = Qué cambió
editor-diff-buffer = en el editor
editor-diff-hint = Esta comparación solo muestra lo que ha cambiado — Esc vuelve a la pregunta


# job journal and saved syncs (0.41)
journal-left = Copias que la última sesión dejó sin terminar: { $count } · Alt+Shift+F6 las lista
journal-title = Trabajos sin terminar
journal-hint = Intro continuar · Supr descartar · Esc cerrar
journal-none = Ningún trabajo quedó sin terminar
journal-row = { $kind }: { $what } → { $target } · archivos hechos: { $done }
journal-row-unreadable = no se puede leer: { $file } ({ $error })
journal-unreadable = { $file }: no es un diario de trabajo que esta versión sepa leer ({ $error }). Alt+Shift+F6 lo lista, y Supr allí lo quita
journal-resumed = Se continúa { $kind }: { $done } archivos ya estaban hechos
journal-discarded = El trabajo sin terminar se descartó, con lo que había escrito a medias
journal-cannot-resume = Ese diario no se puede leer; Supr lo quita
menu-jobs-left = Trabajos sin terminar
sync-report = Escribir un informe de la ejecución en (un archivo)
sync-name = Guardar este par como (un nombre)
sync-saved-button = Guardadas…
sync-saved-title = Sincronizaciones guardadas
sync-saved-hint = Intro ejecutar · F4 cambiar · Supr olvidar · Esc cerrar
sync-saved-none = Aún no hay ninguna sincronización guardada: dele un nombre en el diálogo (Ctrl+Shift+Y)
sync-saved-removed = { $name } ya no está guardada
sync-report-local = El informe tiene que ser un archivo de esta máquina
sync-report-written = El informe de la sincronización está en { $path }
sync-report-not-written = No se pudo escribir el informe en { $path }: { $error }
sync-report-title = Informe de sincronización
sync-report-name = Nombre: { $name }
sync-report-left = Izquierda: { $path }
sync-report-right = Derecha: { $path }
sync-report-to-right = Copiados de izquierda → a derecha: { $count }
sync-report-to-left = Copiados de derecha → a izquierda: { $count }
sync-report-cut-right = Quitados a la derecha: { $count }
sync-report-cut-left = Quitados a la izquierda: { $count }
sync-report-skipped = Dejados como están aunque difieren: { $count }
sync-report-failed = Fallidos: { $count }
sync-report-stopped = Detenido a medias, así que estos pueden haberse hecho o no: { $count }
sync-report-not-done = No hechos, porque un paso anterior falló o fue detenido: { $count }
menu-sync-saved = Sincronizaciones guardadas

# hex editor search
hex-search-wrapped = `{ $query }`: encontrado al empezar de nuevo desde el otro extremo del archivo
hex-replace-title = Sustituir
hex-replace-with = Sustituir por
btn-replace = Sustituir
hex-replace-ask = ¿Sustituir { $was } en { $at } por { $with }?
hex-replace-length = la sustitución mide { $with } bytes y la coincidencia { $was }: desplazaría todo lo que sigue — pulsa Ins para pasar antes al modo de inserción

# pixels and video (0.41)
opt-viewer-pixels = Imágenes
opt-viewer-pixels-auto = píxeles reales donde el terminal los dibuja
opt-viewer-pixels-kitty = protocolo gráfico de kitty
opt-viewer-pixels-iterm = imágenes en línea de iTerm2
opt-viewer-pixels-sixel = sixel
opt-viewer-pixels-off = siempre medios bloques
video-needs-program = esto es un vídeo ({ $what }): sus fotogramas los dibuja `{ $program }`, que no está instalado en esta máquina — F11 sigue leyendo el contenedor
video-no-frame = ningún fotograma de este vídeo: { $why }
video-playing = reproduciendo a { $fps } fotogramas por segundo, sin sonido — Espacio pausa
video-paused = en pausa en { $at } — Espacio continúa, RePág y AvPág saltan una décima
video-ended = el vídeo ha terminado — Espacio lo reproduce de nuevo


# finding things
menu-saved-searches = Búsquedas guardadas
drive-searches = Búsquedas guardadas
find-keys = F2 guarda esta búsqueda con un nombre · F9 las guardadas
find-save-name = Guardar esta búsqueda como
find-saved = búsqueda guardada como { $name }
find-saved-none = aún no hay búsquedas guardadas: F2 en el diálogo de búsqueda guarda una
find-saved-title = Búsquedas guardadas — Intro abre, F4 edita, Supr olvida
find-saved-running = buscando { $name }…
find-saved-gone = búsqueda { $name } olvidada
find-loaded = búsqueda { $name } cargada
find-fuzzy = Nombre aproximado: las letras de arriba, en orden, el mejor primero
find-meta = Metadatos (artist=bowie; camera=)
find-index = Preguntar al índice del sistema en vez de recorrer (puede no ver archivos nuevos)
tools-for-index = el índice que Alt+F7 puede consultar en vez de recorrer
menu-frecent = Carpetas frecuentes
frecent-none = aún no se ha visitado ninguna carpeta
frecent-gone = { $path } quitada de las carpetas frecuentes
frecent-hint = escribe letras de la ruta · Intro ir · Supr olvidar · Esc cerrar

# ai third round
ai-redact-off = El enmascarado está apagado: todo sale tal cual
ai-redact-none = Ningún secreto que enmascarar
ai-redact-count = Secretos enmascarados antes de enviar: { $count } ({ $kinds })
ai-redacted = secretos enmascarados: { $count }
ai-preview-title = Lo que se envía a { $name } ({ $host })
ai-preview-instruction = Instrucción
ai-preview-earlier = La conversación hasta ahora
ai-preview-question = La pregunta y sus archivos
ai-preview-key = F3 muestra lo que se envía
ai-preview-reading = Leyendo lo que se enviaría…
key-ai-redact = Enmasc
ai-redact-now-on = Enmascarado encendido: claves, tokens, claves privadas y contraseñas se enmascaran antes de enviar
ai-redact-now-off = Enmascarado apagado: lo que se envía sale tal cual
ai-panel-redact-on = secretos enmascarados
ai-panel-redact-off = secretos sin enmascarar
key-ai-local = Local
ai-row-refused = no es esta máquina: rechazado
ai-panel-local-only = solo local
ai-local-now-on = Solo local encendido: un servicio que no está en esta máquina se rechaza, y no se le envía nada
ai-local-now-off = Solo local apagado: se puede preguntar a cualquier servicio de la lista
ai-local-refused = Solo local está encendido: { $name } responde en { $host }, que no es esta máquina — no se envió nada (F6 en el panel AI lo apaga)
