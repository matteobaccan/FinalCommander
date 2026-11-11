# Key bar labels: short, they must fit in a few columns.
key-help = Ajuda
key-menu = MenUsu
key-view = Ver
key-edit = Editar
key-copy = Copiar
key-print = Impr
key-move = RenMov
key-mkdir = CrDir.
key-delete = Apagar
key-pulldown = ConfMn
key-quit = Sair
# The key bar while a modifier is held: FAR's own words, six columns wide.
key-add = Adic.
key-extract = Desc.
key-arccmd = CmdArq
key-edit-new = Editar…
key-rename = Renom.
key-group = Grupo
key-selup = SelAc
key-left = Esq.
key-right = Dir.
key-sort-name = Nome
key-sort-ext = Extens
key-sort-write = Modif.
key-sort-size = Tam.
key-sort-unsort = S/Clas
key-sort-create = Criaç.
key-sort-access = Acesso
key-sort = Class.
key-mklink = CrLink
key-find = Local.
key-history = Histór
key-folders = HisDir
key-plugins = Plugin

# Panel chrome
list-up = Acima
list-folder = Diret
col-name = Nome
col-ext = Ext
col-size = Tamanho
col-allocated = Alocado
col-modified = Modificado
col-created = Criado
col-accessed = Acessado
col-attr = Atrib
col-owner = Dono
col-group = Grupo
col-descr = Descrição
col-links = Lnk
col-git = Git
descr-title = Descrição de { $name }
descr-set = { $name } descrito
descr-cleared = descrição de { $name } removida
descr-not-here = as descrições ficam ao lado de um diretório nesta máquina
col-date-time = Data       Hora
panel-list-count = { $count } itens na lista
panel-empty = (vazio)
panel-loading = lendo…

# Prompts
prompt-select-mask = Selecionar arquivos pela máscara:
prompt-deselect-mask = Desmarcar arquivos pela máscara:
prompt-goto = Mudar para o diretório:
prompt-filter = Máscara de arquivo ou parte de um nome:
prompt-search = Procura rápida:

# Dialogs
dialog-error = Erro
dialog-help-hint = Esc volta ao índice · F7 procura em todos os capítulos · F1 fecha · ↑↓ PgUp PgDn rolam
dialog-help-index = FinalCommander { $version } — Ajuda
dialog-help-hint-index = Enter abre um capítulo · F7 procura em todos os capítulos · Esc fecha
help-search-prompt = Procurar na ajuda
help-search-none = Nenhuma linha da ajuda contém “{ $query }”
help-search-title = “{ $query }” na ajuda: { $count } linhas
output-search-prompt = Procurar na saída
output-search-empty = Nada foi impresso ainda
output-search-found = Linha { $line } de { $lines }
output-search-wrapped = Linha { $line } de { $lines }, de novo a partir da mais recente
output-search-none = Nenhuma linha da saída contém “{ $query }”

# Status messages
msg-not-implemented = { $command } ainda não está implementado nesta versão
msg-selected = { $count } itens selecionados
msg-deselected = { $count } itens desmarcados
msg-mask-invalid = máscara inválida: { $error }
msg-theme = tema: { $name }
msg-font-bigger = fonte: uma maior foi pedida ao terminal
msg-font-smaller = fonte: uma menor foi pedida ao terminal
msg-font-reset = fonte: a inicial foi pedida ao terminal
msg-sort = classificação: { $key }{ $reverse }
msg-sort-groups-on = grupos de classificação ligados ({ $count })
msg-sort-groups-off = grupos de classificação desligados ({ $count })
msg-sort-groups-none = nenhum grupo de classificação definido: veja [[sort_group]] na configuração
msg-view = visualização: { $mode }
msg-hidden-on = arquivos ocultos mostrados
msg-hidden-off = arquivos ocultos escondidos
msg-no-history = não há mais histórico
msg-tab-last = um painel sempre tem uma aba
msg-tab-end = a aba já está nessa ponta da fila
tabs-title = Abas
msg-tab-moved = a aba foi para o outro painel
msg-tab-locked = esta aba está travada no seu diretório
msg-tab-locked-on = aba travada no seu diretório
msg-tab-locked-off = aba destravada
msg-bookmark-set = atalho { $digit }: { $path }
msg-bookmark-unset = o atalho { $digit } não está definido
msg-bookmark-added = { $path } guardado
msg-bookmark-removed = { $path } removido
msg-bookmark-known = esse diretório já está na lista
msg-dirsize = { $path }: { $bytes } em { $files } arquivos
msg-dirsize-many = medindo { $count } diretórios…
usage-title = Uso do disco: { $path } · { $size } · Tab: a árvore inteira como mapa
usage-not-here = o uso do disco de um diretório é medido num disco, não dentro de um arquivo compactado nem numa lista de arquivos encontrados
msg-dirsize-no-folder = não há aqui nenhum diretório para medir

# Jobs: prompts, confirmations, progress, conflicts
prompt-copy-to = Copiar { $what } para:
prompt-move-to = Renomear ou mover { $what } para:
prompt-mkdir = Criar o diretório:
prompt-tab-name = Nome desta aba:
prompt-bookmark-name = Nome deste diretório:
what-selected = { $count } itens selecionados
confirm-delete-title = Apagar
confirm-delete-body =
    { $count ->
        [one] Deseja apagar { $name } ({ $bytes })?
       *[other] Deseja apagar { $count } itens ({ $bytes })?
    }
prompt-link = Criar um link para { $name } como:
link-created = { $name } criado
confirm-wipe-title = Limpar
confirm-wipe-body =
    { $count ->
        [one] Sobrescrever e apagar { $name } ({ $bytes })?
       *[other] Sobrescrever e apagar { $count } itens ({ $bytes })?
    }
confirm-wipe-honest = Os bytes são sobrescritos uma vez e depois o arquivo é removido. Num SSD, num sistema de arquivos copy-on-write ou com snapshots, cópias antigas podem sobreviver — e o nome permanece no diretório e no journal.
job-copy = Copiando
job-move = Movendo
job-delete = Apagando
job-wipe = Limpando
job-trash = Movendo para a lixeira
job-replace = Substituindo
job-pack = Compactando
job-rebuild = Gravando o arquivo compactado

# ── the attributes dialog (SPEC 10 §3.8) ───────────────────────────────────────────────
attr-title = Atributos do arquivo
attr-count = { $count } itens selecionados
attr-now = agora { $text } ({ $octal })
attr-owner-read = O dono pode ler
attr-owner-write = O dono pode gravar
attr-owner-exec = O dono pode executar
attr-group-read = O grupo pode ler
attr-group-write = O grupo pode gravar
attr-group-exec = O grupo pode executar
attr-other-read = Todos podem ler
attr-other-write = Todos podem gravar
attr-other-exec = Todos podem executar
attr-setuid = Executar como o dono (setuid)
attr-setgid = Executar como o grupo (setgid)
attr-sticky = Só o dono pode apagar (sticky)
attr-readonly = Somente leitura
attr-hidden = Oculto
attr-system = Sistema
attr-archive = Arquivo
attr-temporary = Temporário
attr-offline = Offline
attr-indexed = O indexador pode examiná-lo
attr-compressed = compactado
attr-encrypted = criptografado
attr-sparse = esparso
attr-reparse = um reparse point
attr-also = também { $what }
attr-mtime = Modificado
attr-atime = Última leitura
attr-btime = Criado
attr-ctime = Registro alterado
attr-btime-readonly = (a hora de criação só pode ser definida no Windows)
attr-utc = Horas em UTC
attr-owner = Dono (usuário:grupo)
attr-owner-is = dono { $who }
btn-attr-original = Original
btn-attr-current = Atual
btn-attr-blank = Limpar
attr-recursive = E tudo o que há dentro dos diretórios
attr-bad-time = `{ $text }` não é uma data e hora (AAAA-MM-DD HH:MM:SS)
attr-bad-owner = não há aqui conta nem grupo chamado `{ $text }`
attr-in-archive = esta alteração regrava o arquivo compactado, com todas as outras entradas como estão
job-attrs = Ajustando atributos

# ── the copy dialog (SPEC 03 §4) ───────────────────────────────────────────────────────
copy-title = Copiar
copy-title-move = Renomear/Mover
copy-target = Para
copy-conflict = Se o arquivo já existe
copy-conflict-ask = perguntar
copy-conflict-overwrite = substituir
copy-conflict-newer = substituir quando a origem for mais nova
copy-conflict-skip = pular
copy-conflict-rename = dar à cópia um nome livre
copy-conflict-append = acrescentar ao final
copy-only-newer = Somente arquivos mais novos que os de lá
copy-filter = Somente os nomes que combinam
copy-follow-links = Copiar o que o link aponta, não o link
copy-keep-times = Preservar as datas e horas
copy-keep-mode = Preservar as permissões
copy-verify = Reler cada arquivo e compará-lo
copy-several = { $count } destinos, um após o outro

# ── the terminal (SPEC 06 §4) ──────────────────────────────────────────────────────────
term-started = um terminal executando { $shell }
term-hint = Ctrl+Alt+E: voltar aos painéis
term-ended = terminou — qualquer tecla fecha
job-mkdir = Criando diretório
job-planning = planejando…
job-counters = { $files } / { $filestotal } arquivos · { $bytes } / { $bytestotal }
job-rates = { $speed }/s · { $elapsed } decorridos · faltam { $eta }
job-rates-no-eta = { $speed }/s · { $elapsed } decorridos
job-hint = Esc cancela
job-hint-queue = Esc cancela · B segundo plano
job-queued = { $kind } está na fila, com { $ahead } outras operações não terminadas · Alt+Shift+F5 lista
job-background = { $kind } continua em segundo plano · Alt+Shift+F5 lista as operações
job-asks = { $kind } aguarda uma resposta · Alt+Shift+F5
job-paused = { $kind } em pausa
job-resumed = { $kind } continua
job-retry-nothing = Nada para repetir
job-still-waiting = Essa operação está esperando a sua vez
jobs-title = Operações ({ $active } não terminadas)
jobs-hint = Enter mostrar · Space pausar · Del parar · F5 repetir · Ctrl+↑↓ ordem · Esc fechar
jobs-quit-title = Ainda há operações em andamento
jobs-quit-body = Estas operações param se o programa terminar agora:
job-state-running = em andamento
job-state-paused = em pausa
job-state-asking = perguntando
job-state-queued = aguardando
job-state-held = retida
job-state-done = concluída
job-state-failed = erros
job-state-cancelled = cancelada
job-done = { $kind }: { $files } arquivos, { $bytes }
job-done-renamed = { $kind }: { $files } arquivos movidos sem copiar
job-done-skipped = , { $skipped } pulados
job-done-verified = , { $verified } verificados
job-done-unverified = , { $unverified } não puderam ser relidos aqui
job-done-errors = , { $errors } erros
job-cancelled = { $kind }: cancelado
job-errors-title = Erros
conflict-title = O arquivo já existe
conflict-dst = destino  { $path }   { $bytes }
conflict-src = origem   { $path }   { $bytes }
conflict-newer = a origem é mais nova
conflict-older = o destino é mais novo
btn-overwrite = Substituir
btn-overwrite-all = Todos
btn-skip = Pular
btn-skip-all = Pular todos
btn-retry = Repetir
btn-move = Mover
btn-delete = Apagar
btn-wipe = Limpar
btn-copy = Copiar
btn-discard = Descartar
btn-kill = Encerrar
btn-print = Imprimir
btn-quit = Sair
btn-play = Executar
btn-run = Rodar
btn-accept = Aceitar
btn-cancel = Cancelar
msg-nothing-to-do = nada selecionado
cmd-complete-nothing = nada com que completar isso
complete-several = { $count } nomes começam assim
form-moving = as setas movem o diálogo · Enter ou Esc o deixa ali
form-moved = o diálogo fica onde você o colocou
msg-trash-refused = A lixeira não aceitou — Shift+Del apaga de vez.

# Checksums (SPEC 03 §9)
sum-title = Somas de verificação
sum-algorithm = Hash
sum-one-file = Uma lista para todos
sum-name = Nome
sum-working = { $name }: { $done } de { $total }
sum-verifying = verificando com { $name }
sum-stopped = interrompido
sum-written = { $name } gravado
sum-no-name = a lista precisa de um nome
sum-local-only = uma soma de verificação só pode ler um arquivo desta máquina
print-title = Imprimir
print-body = Imprimir {$what}?
print-sending = enviando para a impressora…
print-sent = enviado para a impressora
print-failed = a impressão falhou ({$line}): {$error}
print-local-only = só é possível imprimir um arquivo desta máquina
print-no-command = nenhum comando de impressão: defina [print] command ou instale lp ou lpr
sum-all-match = todos os { $count } conferem
sum-verify-title = Somas de verificação
sum-verify-summary = { $ok } conferem, { $bad } não
sum-differs = não confere
sum-missing = ausente
sum-unreadable = não pode ser lido

# Synchronising two directories (SPEC 03 §10)
sync-title = Sincronizar
sync-between = { $left }  ↔  { $right }
sync-recursive = Também tudo nos subdiretórios
sync-both-ways = Nos dois sentidos, vence o mais novo
sync-delete-extra = Oferecer a remoção do que o outro lado não tem
sync-by-content = Comparar os bytes, não o tamanho e a hora
sync-filter = Somente os nomes que combinam
sync-walking = examinando { $at } — { $pairs } até agora
sync-planned = { $copies } para copiar ({ $bytes }), { $deletes } para remover
sync-hint = Space próxima resposta · ← → sentido · = deixar · F2 executar · Esc cancelar
sync-hint-walking = Esc interrompe a varredura
sync-nothing-to-do = todas as linhas dizem para deixar tudo como está
sync-local-only = uma sincronização só pode ler e gravar nesta máquina
sync-same-directory = os dois painéis estão no mesmo diretório

# What to run for a kind of file (SPEC 06 §5)
assoc-none = nada aqui diz como abrir { $name }
usermenu-empty = Ainda não há menu do usuário: escreva um em { $path }
usermenu-last-gone = a linha escolhida da última vez não está mais neste menu
usermenu-bad-condition = Condição não entendida — { $problem }
assoc-desktop = Entregar à área de trabalho
assoc-open-with = Abrir { $name } com
assoc-no-pager = nenhuma associação nem $PAGER diz com que ler isto
assoc-no-editor = nenhuma associação nem $VISUAL diz com que editar isto

# Viewer (F3)
key-wrap = Quebra
key-hex = Hex
key-text = Texto
key-dump = Dump
key-doc = Doc
key-search = Local.
key-code = Código
key-follow = Seguir
viewer-prompt-search = Procurar por ({ $mode }):
viewer-prompt-goto = Ir para o deslocamento, ou NN% :
viewer-not-found = `{ $query }` não encontrado
viewer-follow-on = seguindo o final do arquivo
viewer-follow-off = modo seguir desligado
viewer-codepage = página de código: { $name }
viewer-wrap-mode = { $mode }
viewer-no-undo = nenhuma posição anterior
viewer-empty = (arquivo vazio)
viewer-not-markdown = não é um arquivo Markdown, ou é grande demais para formatar
viewer-not-table = não é um arquivo tabular, ou é grande demais para ler inteiro
viewer-base64-done = Decodificado: { $bytes }
viewer-not-base64 = Este arquivo não é base64: { $why }
viewer-status = { $mode } { $codepage }  { $offset } / { $len }  { $percent }%

key-disasm = Disasm
viewer-disasm-failed = não foi possível desmontar { $name }: { $error }
viewer-decompile-running = { $tool } está processando { $name }…
viewer-decompile-none = nada aqui consegue descompilar { $format }; instale uma ferramenta e adicione uma regra [[decompile]] ao fcmd.toml
arc-needs-program = este arquivo compactado é lido por `{ $program }`, que não está instalado nesta máquina
viewer-decompile-failed = { $tool }: { $error }
viewer-blame-derived = o git blame é do próprio arquivo: volte a ele primeiro
viewer-derived = { $producer } de { $name }

# Editor (F4)
key-save = Salvar
key-replace = Subst.
editor-prompt-goto = Ir para a linha, ou linha:coluna :
editor-prompt-save-as = Salvar arquivo como:
editor-saved = { $name }: salvo, { $bytes } bytes
editor-nothing-to-save = { $name } não tem alterações não salvas
editor-not-found = `{ $query }` não encontrado
editor-replaced = { $count } substituições
editor-read-only = { $name } é somente para leitura
editor-readonly-on = somente leitura: as edições são recusadas
editor-readonly-off = somente leitura desligado
editor-no-undo = nada mais para desfazer
editor-no-redo = nada para refazer
editor-discard-title = Alterações não salvas
editor-discard-body = { $name } tem alterações não salvas. Sair e perdê-las?
editor-changed-title = Modificado no disco
editor-changed-body = { $name } foi modificado no disco desde que foi aberto. Salvar por cima?
editor-reloaded = { $name } recarregado do disco
editor-failed = { $name }: { $error }

# netbox (SPEC 04 §6)
net-sites-title = Conexões
net-sites-hint = Enter conectar · n URL · Ins guardar · F4 editar · d desconectar · Esc fechar
net-no-sites = Ainda não há sites. Pressione `n` para digitar uma URL, ou adicione blocos [[site]] à configuração.
net-not-a-stored-session = esta linha é uma conexão aberta, não uma sessão guardada
net-site-forgotten = { $name } não está mais guardado
net-prompt-url = Conectar a:
net-prompt-password = Senha de { $user }@{ $host }:
net-prompt-passphrase = Frase secreta da chave de { $user }@{ $host }:
net-connecting = conectando a { $host }…
net-connected = conectado a { $host }
net-disconnected = { $host } desconectado
net-not-connected = { $host } não está conectado
net-local-panel = este painel está num sistema de arquivos local
net-nothing-pending = nenhuma conexão está esperando por isso
net-hostkey-title = Chave de host desconhecida
net-hostkey-body = { $host } não está em known_hosts. A impressão digital é:
net-hostkey-question = Registrá-la e conectar?
net-hostkey-changed-title = A chave do host MUDOU
net-hostkey-changed-body = { $host } respondeu com uma chave que não é a registrada.
net-hostkey-changed-where = A registrada está em { $path }, linha { $line }.
net-hostkey-changed-what = Um servidor reinstalado é idêntico a alguém no meio do caminho. Nada é aceito aqui: abra o arquivo, leia a linha e remova-a você mesmo se souber por que mudou.
btn-open-file = Abrir o arquivo
net-fetching = baixando { $name }…
net-viewing-copy = { $name }: uma cópia local, baixada para o visualizador
net-too-big = { $name } é maior que { $limit }: aumente net.fetch_limit_mb para baixá-lo
net-editing = { $name }: editando uma cópia local; ao salvar, ela é gravada de volta
net-writing-back = gravando { $name } de volta…
net-written-back = { $name }: gravado de volta, { $bytes } bytes
net-write-back-failed = a gravação de volta falhou: { $error } — suas edições estão em { $local }
net-changed-title = Modificado no servidor
net-changed-body = { $name } foi modificado no servidor desde que foi baixado. Gravar por cima?
confirm-delete-permanent = Isto apaga definitivamente: não há lixeira para estes itens.
confirm-trash-title = Mover para a lixeira
confirm-trash-body =
    { $count ->
        [one] Deseja mover { $name } ({ $bytes }) para a lixeira?
       *[other] Deseja mover { $count } itens ({ $bytes }) para a lixeira?
    }
confirm-trash-where = Eles vão para onde a área de trabalho guarda os arquivos apagados e podem ser restaurados de lá.

# ---------------------------------------------------------------- menu bar (SPEC 10)
menu-left = &Esquerda
menu-right = &Direita
menu-files = &Arquivos
menu-commands = &Comandos
menu-options = &Opções
menu-brief = &Simples
menu-medium = &Média
menu-full = &Completa
menu-detailed = Deta&lhada
menu-owners = Do&nos dos arquivos
menu-links = Lin&ks do arquivo
menu-alt-full = Completa, taman&hos exatos
menu-descriptions = Descri&ções
menu-long-descriptions = Descrições lon&gas
menu-wide = L&onga
menu-info-panel = Painel de &inform.
menu-quick-view = &Visualiz. rápida
menu-tree-panel = &Painel de árvore
menu-tab-new = Nova &aba
menu-tab-close = Fechar aba
menu-tab-next = Próxima a&ba
menu-tab-prev = Aba an&terior
menu-tab-move = Mover a aba para o o&utro painel
menu-tab-lock = Fi&xar a aba no seu diretório
menu-tab-name = Dar nome a esta aba
menu-tab-list = Todas as abas
menu-sort-modes = Modos de classi&f.
menu-hidden = Arquivos ocultos e de sistema
menu-filter = Filtro de ar&quivos
menu-filter-sets = Filtros com nome
menu-panel-onoff = Liga/D&esl. painel
menu-reread = &Re-ler
menu-change-drive = Alterar unida&de
menu-view = &Ver
menu-edit = &Editar
menu-edit-new = Editar &novo arquivo
menu-copy = &Copiar
menu-rename = &Renomear ou mover
menu-link = Criar lin&k
menu-multirename = Renomear por &padrão…
menu-multirename-undo = Desfazer a última renomeação em lote
menu-mkdir = Criar d&iretório
menu-delete = &Apagar
menu-wipe = &Limpar
menu-checksum = &Somas de verificação…
menu-checksum-verify = Verificar com uma lista
menu-split = Dividir arquivo…
menu-combine = Juntar arquivos
menu-delete-permanent = Apagar de&finitivamente
menu-archive-add = Adic. ao ar&quivo compactado
menu-archive-extract = &Descompactar arquivos
menu-archive-commands = C&omandos de compactação
menu-archive-test = Testar arquivo compactado
menu-attributes = A&tributos do arquivo
menu-apply-command = Aplicar co&mando
menu-replace-in-files = Substituir nos arquivos…
menu-select-group = Selecionar &grupo
menu-select-ext = Selecionar por esta e&xtensão
menu-unselect-ext = Desmarcar por esta extensão
menu-unselect-group = Desmarcar gr&upo
menu-invert-selection = Inverter sele&ção
menu-select-all = Selecionar tudo
menu-restore-selection = Restaurar seleç&ão
menu-dirsize = Taman&ho do diretório
menu-find-file = &Procurar arquivo
menu-history = &Histórico de comandos
menu-view-history = H&ist. de visualização e edição
menu-folders-history = Histórico de &diretórios
menu-usage = Uso do disco
menu-jobs = Opera&ções
menu-branch = Visão de ramo
menu-bookmarks = Atalhos de diretório
menu-swap-panels = T&rocar painéis
menu-sync-panel = Sincroni&zar o outro painel
menu-panels-onoff = &Liga/Desl. painéis
menu-compare-folders = &Comparar diretórios
menu-sync-folders = &Sincronizar diretórios
menu-compare-contents = Comparar por conteúd&o
menu-panelize = Saída de um comando no painel
menu-view-filtered = Ver através de um comando
menu-learn-keys = Aprender teclas
menu-tunnel = A&brir um túnel…
menu-tunnels = Tún&eis…
tunnel-title = Túnel: [local:]host:porta
tunnel-list-title = Túneis
tunnel-none = nenhum túnel está aberto
tunnel-open = túnel aberto: { $what }
tunnel-closed = túnel fechado
tunnel-failed = { $what }: { $why }
tunnel-needs-connection = um túnel passa por uma conexão: abra uma com Alt+N primeiro
menu-connections = Conexões
menu-disconnect = Desconectar
menu-user-menu = Menu do &usuário
menu-associations = Assoc. de ar&quivos
menu-folder-shortcuts = A&talhos de diretório
menu-plugin-commands = Comandos de plu&gin
menu-screens-list = List&a de telas
menu-macro-record = Gravar uma macro
menu-macro-list = Macros
menu-task-list = Lista de tare&fas
menu-system-settings = Configurações do &sistema
menu-panel-settings = Configurações do &painel
menu-interface-settings = Configurações da &interface
menu-languages = I&diomas
menu-themes = &Temas
menu-confirmations = Co&nfirmações
menu-viewer-settings = Configurações do &visualizador
menu-editor-settings = Configurações do &editor
menu-codepages = Pá&ginas de código
menu-sort-groups = Grupos de c&lassificação
menu-mask-groups = &Máscaras de arquivos
menu-settings-all = Tod&as as configurações
menu-highlighting = Desta&car arquivos
menu-plugins-config = Configuração de pl&ugins
menu-help = Ajuda
menu-save-setup = Salvar con&figuração
menu-quit = Sai&r
opt-natural-sort = Tratar dígitos como números
opt-allow-reverse = Permitir classificação inversa
opt-select-folders = As máscaras selecionam diretórios também
cmd-no-network-name = Um caminho neste disco não tem nome de rede
opt-trash = Apagar para a lixeira
opt-system-copy = Usar a rotina de cópia do sistema
opt-save-history = Manter o histórico entre sessões
opt-history-lifetime = Duração do histórico, em dias
opt-forever = para sempre
opt-fetch-limit = Limite de tamanho de arquivo remoto
opt-keepalive = SSH keep-alive
opt-off = desligado
opt-show-hidden = Mostrar arquivos ocultos e de sistema
opt-column-titles = Mostrar títulos das colunas
opt-status-line = Mostrar linha de status
opt-scrollbar = Mostrar barra de rolagem
opt-free-space = Mostrar espaço livre
opt-auto-refresh = Atualizar o painel automaticamente
opt-dirs-first = Diretórios primeiro
opt-selected-first = Arquivos selecionados primeiro
opt-reverse = Ordem inversa
opt-view = Visualização do painel
opt-sort = Classificar por
opt-quick-search = Procura rápida
opt-quick-alt = com Alt
opt-quick-plain = pelas letras
opt-key-bar = Mostrar barra de botões
opt-status-bar = Mostrar barra de status
opt-syntax = Colorir as linguagens que reconhecemos
opt-markdown = Markdown formatado para leitura
opt-table = Abrir .csv ou .jsonl como tabela
opt-keymap = Mapa de teclas
opt-clock = Relógio no canto
opt-mouse = Mouse
opt-clipboard = Deixar o terminal transportar a área de transferência (OSC 52)
opt-command-screen = Mostrar a saída de um comando enquanto ele roda
opt-menu-always = Mostrar sempre a barra de menus
opt-save-exit = Salvar a configuração ao sair
opt-confirm-delete = Apagar
opt-confirm-overwrite = Substituir
opt-confirm-exit = Sair
opt-confirm-editor = Sair do editor com alterações não salvas
opt-applied = configurações aplicadas
opt-saved = salvo em { $path }
opt-set-title = { $key }
opt-set = { $key } = { $value }
opt-not-a-toggle = Não é uma configuração de sim ou não: Enter digita um valor
opt-settings-hint = ↑↓ escolher · digite para filtrar · Enter ou F4 valor · Space alternar · Del padrão · Esc fechar
opt-language-set = idioma: { $name }
sort-name = Nome
sort-ext = Extensão
sort-size = Tamanho
sort-mtime = Hora de modificação
sort-btime = Hora da criação
sort-atime = Hora de acesso
sort-descr = Descrição
sort-owner = Dono
sort-git = Estado no Git
key-sort-descr = Descr
key-sort-owner = Dono
sort-none = Sem classificação
sort-set = classificação: { $name }
btn-ok = OK
pick-hint = ↑↓ escolher · digite para filtrar · Enter aplicar · Esc fechar
pick-empty = (nada para escolher)
# ------------------------------------------------- the three histories (SPEC 06 §6)
hist-pinned = { $line } está fixado · agora nada o empurra para fora
hist-unpinned = { $line } não está mais fixado
hist-is-pinned = { $line } está fixado · Ins o desafixa primeiro
hist-clear-body = Esvaziar este histórico de tudo o que não está fixado?
hist-cleared = o histórico está vazio, exceto pelo que está fixado
history-hint = ↑↓ escolher · Enter aplicar · Ins fixar · Shift+Del um · Del todos · Esc fechar


# ------------------------------------------------- session editor (SPEC 10 §3.5)
site-new = Nova conexão
site-edit = Editar conexão
site-name = Nome
site-host = Host
site-port = Porta
site-user = Usuário
link-title = Criar um link
link-name = O link
link-kind = Tipo
link-kind-relative = simbólico, relativo — sobrevive se a árvore for movida
link-kind-absolute = simbólico, absoluto — sobrevive se o link for movido
link-kind-hard = hard link — um segundo nome para o mesmo arquivo
menu-colors = C&ores…
palette-title = Cores de { $theme }
palette-hint = Enter uma cor · F2 salvar · Esc fechar
palette-ask = Uma cor para { $key }
palette-bad-color = { $value } não é uma cor (um nome, 0–255 ou #rrggbb)
palette-saved = as cores foram gravadas no tema { $theme }
palette-unsaved = as cores mudaram só na tela — F2 as grava
opt-viewer-status-line = Mostrar a linha superior (nome e posição)
opt-viewer-scrollbar = Mostrar barra de rolagem
opt-viewer-key-bar = Mostrar barra de botões
opt-viewer-wrap = Linhas longas
opt-viewer-wrap-off = não quebrar
opt-viewer-wrap-char = quebrar na borda
opt-viewer-wrap-word = quebrar na palavra
opt-viewer-tab-width = Largura da tabulação
opt-viewer-remember = Lembrar a posição e a página de código de cada arquivo
opt-editor-tabs = Tabulações
opt-editor-tabs-keep = deixá-las como estão
opt-editor-tabs-typed = expandir o que é digitado
opt-editor-tabs-all = expandir todas ao abrir um arquivo
opt-editor-tab-width = Largura da tabulação
opt-editor-auto-indent = Manter o recuo da linha de cima
opt-editor-whitespace = Mostrar espaços e tabulações
opt-editor-cursor-after = O cursor fica depois do texto que a procura encontrou
opt-editor-beyond-eol = O cursor pode passar do fim da linha
opt-editor-trim-trailing = Remover espaços no fim das linhas ao salvar
editor-not-a-character = essa tecla não tem caractere para inserir
paste-one-line =
    { $count ->
        [one] só a primeira linha foi colada · mais uma linha ficou de fora
       *[other] só a primeira linha foi colada · mais { $count } linhas ficaram de fora
    }
clip-names =
    { $count ->
        [one] um nome na área de transferência
       *[other] { $count } nomes na área de transferência
    }
clip-paths =
    { $count ->
        [one] um caminho na área de transferência
       *[other] { $count } caminhos na área de transferência
    }
clip-line = a linha de comando está na área de transferência
clip-selection = o que estava marcado está na área de transferência
clip-off = a área de transferência está desligada — `ui.clipboard` nas configurações
history-gone = { $name } não está mais lá
hl-new = Novo grupo de cores
hl-edit = Grupo de cores
# ------------------------------------------------- the diff screen (SPEC 05 §3)
diff-title = Diferenças
menu-diff = &Ver diferenças entre arquivos
diff-needs-two = Ponha o cursor num arquivo em cada painel
diff-not-here = { $name } não é um arquivo neste disco
diff-too-big = { $name } é maior que { $most }
diff-same = Os dois arquivos são iguais
diff-where = trecho { $at } de { $count } · -{ $removed } +{ $added } linhas
diff-no-next = A última diferença
diff-no-previous = A primeira diferença
diff-hint = Alt+↑↓ trechos · Tab lado · F5 levar · F2 gravar · U unificado · Shift+F2 patch · W C E ignorar · Esc
diff-hint-git = Alt+↑↓ trechos · S preparar o trecho · Tab lado · F5 levar · F2 gravar · Esc fechar
diff-hint-git-staged = Alt+↑↓ trechos · S retirar o trecho · Tab lado · F5 levar · F2 gravar · Esc fechar
diff-stage-edited = o diff tem mesclagens não gravadas: F2 primeiro, depois S
diff-merged-left = O arquivo da esquerda recebeu essa diferença
diff-merged-right = O arquivo da direita recebeu essa diferença
diff-no-hunk-here = Não há diferença no topo da tela — Alt+↓ põe uma ali
diff-nothing-to-write = Nada foi mesclado
diff-written = { $count } gravados
diff-unwritten = Uma mesclagem não foi gravada — F2 grava, outro Esc a abandona
diff-ignore-space-on = Espaços ignorados
diff-ignore-space-off = Os espaços voltam a contar
diff-ignore-case-on = Maiúsculas/minúsculas ignoradas
diff-ignore-case-off = Maiúsculas/minúsculas voltam a contar
diff-ignore-eol-on = Fins de linha ignorados
diff-ignore-eol-off = Os fins de linha voltam a contar

grab-hint = ↑↓←→ mover · Shift+↑↓←→ marcar · Ctrl+A tudo · Enter copiar · Esc fechar
grab-copied = { $rows } linhas da tela copiadas

# ------------------------------------------------- the code pages (SPEC 05 §1.12)
cp-title = Páginas de código
cp-name-title = Como chamar esta página de código
cp-set = Lendo como { $name }
menu-panel-names = Página de código dos nomes…
names-title = Ler os nomes como
names-system = Como o sistema os fornece
names-set = Nomes lidos como { $name }
names-system-set = Nomes como o sistema os fornece
names-not-here = Neste sistema um nome não é feito de bytes, então não há outra forma de lê-lo
cp-marked = { $name } agora é uma das que F8 percorre
cp-unmarked = { $name } não é mais uma das que F8 percorre
cp-hint = ↑↓ escolher · Enter ler assim · Ins marcar para F8 · F4 renomear · Esc fechar

# ------------------------------------------------- the named mask groups (SPEC 02 §5b)
mg-title = Grupo de máscaras
mg-name = Nome
mg-mask = Máscaras
mg-hint = Ins adiciona um grupo, Del remove, F4 edita · um `·` marca os três embutidos
mg-saved = <{ $name }> está em vigor; Salvar configuração o grava
mg-removed = <{ $name }> foi removido
mg-bad = <{ $name }>: { $problem }
mg-needs-name = um nome sem espaços, vírgulas nem sinais de menor e maior
mg-needs-mask = um grupo que não representa nada não combina com nada
mg-built-in = <{ $name }> é um dos três embutidos · F4 lhe dá outras máscaras

sg-title = Grupo de classificação
sg-name = Nome
sg-mask = Máscaras
sg-place = Onde
sg-place-front = antes dos demais
sg-place-back = depois dos demais
sg-hint = Ins adiciona um grupo, Del remove, F4 edita, Ctrl+↑↓ move
sg-saved = O grupo está em vigor; Salvar configuração o grava
sg-removed = { $name } foi removido
sg-bad = { $name }: { $problem }
sg-needs-mask = um grupo sem máscara não conteria nada
hl-name = Nome
hl-mask = Máscara ou máscaras
hl-kind = Aplica-se a
hl-kind-both = arquivos e diretórios
hl-kind-file = somente arquivos
hl-kind-dir = somente diretórios
hl-size = Tamanho (1M-10M, -4K, 1G-)
hl-age = Idade em dias (<=1, >=30, 7-30)
hl-date-field = A idade se refere a
hl-attrs = Atributos (h r x s b, - para “não pode ter”)
hl-fg = Cor do texto (#d7875f, red, file.archive)
hl-bg = Fundo, escrito do mesmo jeito
hl-hint = ↑↓ escolher · Enter/F4 editar · Ins novo · Del remover · Ctrl+↑↓ mover · Esc fechar
hl-saved = o grupo está em vigor · Opções → Salvar configuração o grava no arquivo
hl-removed = { $name } foi removido · Opções → Salvar configuração grava isso no arquivo
hl-bad = { $name }: { $problem }
filter-off = Sem filtro: todos os nomes
filter-hint = ↑↓ escolher · Enter usar · F4 editar · Ins novo · Del remover · Esc fechar
filter-new = Novo filtro
filter-edit = Filtro
filter-name-hint = com nome, fica no menu de filtros; sem nome, é usado uma vez
filter-applied = filtro ligado: { $name }
filter-cleared = filtro desligado
filter-saved = { $name } está no painel e no menu de filtros · Opções → Salvar configuração o grava no arquivo
filter-removed = { $name } foi removido · Opções → Salvar configuração grava isso no arquivo
editor-no-block = nada está selecionado
editor-unique = { $count } linhas repetidas removidas
editor-no-bracket = não há parêntese aqui, ou nada combina com ele
editor-tabs-expanded = as tabulações foram expandidas em { $lines } linhas — o salvamento gravará isso
site-protocol = Protocolo
site-protocol-sftp = SFTP (sobre SSH)
site-protocol-ftp = FTP
site-protocol-ftps = FTP com TLS
site-protocol-dav = WebDAV
site-protocol-davs = WebDAV sobre HTTPS
site-auth = Autenticação
site-auth-agent = agente SSH
site-auth-key = arquivo de chave
site-auth-ask = pedir uma senha
site-key-file = Arquivo de chave
site-start-dir = Diretório remoto
site-group = Grupo
site-keepalive = Keep-alive (s)
site-compress = Compressão
site-page-session = Sessão
site-page-environment = Ambiente
site-page-directories = Diretórios
site-page-sftp = SFTP
site-password-note = Nenhuma senha é guardada: ela é pedida ao conectar.
site-env-eol = Converter fins de linha (ainda não)
site-env-utf8 = Forçar nomes em UTF-8 (ainda não)
site-jump = Host de salto
site-jump-note = Alcançado através deste host: host, user@host ou user@host:port. Vazio significa direto ao servidor.
site-sftp-ciphers = Escolher as cifras (ainda não)
btn-connect = Conectar
site-needs-host = uma conexão precisa de um host
site-needs-key = escolha o arquivo de chave, ou outra forma de autenticação
site-bad-port = `{ $port }` não é um número
site-stored = conexão guardada: { $name } (Shift+F9 para mantê-la)
# ------------------------------------------------- structure pane (SPEC 05 §1.8)
key-struct = Estrut
key-markdown = MD
key-table = Tabela
key-filter = Filtro
key-decompile = Decomp
key-next = Próx.
key-prev = Anter.
key-codepages = PágCód
key-goto = IrPara
key-to-panel = Painel
key-blame = Blame
viewer-structure = Estrutura
viewer-structure-hint = ↑↓ linha · Enter ir para · Tab conteúdo · Shift+F11 fechar
viewer-structure-hint-tab = Tab traz as teclas para cá
viewer-no-structure = nada para decodificar: este não é um formato que o painel de estrutura conhece
viewer-no-target = esta linha não aponta para lugar nenhum do arquivo
viewer-form-opened = O formulário, como o texto que o Delphi mostra para ele. F4 volta aos bytes.
viewer-metadata-opened = O texto inteiro, formatado para leitura. F4 volta aos bytes.
viewer-went-to = foi para { $offset }
viewer-in-panel = o painel está em { $name }
viewer-not-in-panel = { $name } não está neste painel
viewer-no-more-files = não há mais arquivos no painel
editor-unsaved-for-viewer = salve primeiro: o visualizador lê o arquivo do disco
# ------------------------------------------------- command line (SPEC 06 §7)
cmd-running = executando: { $command }
prompt-panelize = Comando cuja saída nomeia os arquivos
prompt-viewer-filter = Comando cuja saída será lida no lugar do arquivo
learn-title = Quais teclas este terminal envia
learn-hint = Pressione as teclas. Uma célula acesa chegou; Esc duas vezes sai.
learn-last = última: { $key } — { $command }
learn-unbound = não está associada a nada
recode-title = Regravar como
recode-what = { $count } arquivos
recode-from = O que são agora
recode-to = Gravá-los como
recode-auto = descobrir arquivo por arquivo
recode-eol = Fins de linha
recode-eol-keep = deixar como estão
recode-working = regravando { $count } arquivos
recode-done = { $changed } gravados, { $same } já estavam assim, { $failed } recusados
menu-recode = Regravar como…
panelize-title = { $command }
panelize-done = { $count } arquivos, { $dropped } linhas puladas
panelize-nothing = { $command }: nenhuma linha nomeou um arquivo existente
cmd-interrupted = interrupção enviada ao comando
cmd-done = { $command } terminou
cmd-failed = { $command } saiu com { $code }
cmd-busy = um comando já está em execução
prompt-apply = Comando a executar em cada um dos { $count } arquivos:
cmd-apply-no-ask = um comando que pede um valor não pode ser aplicado arquivo por arquivo
cmd-apply-progress = { $command }  ({ $done } de { $total })
cmd-apply-stopped = interrompido após { $done } de { $total }: { $command } falhou
cmd-not-remote = um comando precisa de um diretório local; este painel está num servidor
cmd-no-history = o histórico de comandos está vazio
cmd-no-previous = não há para onde voltar
cmd-no-such-dir = { $path }: diretório inexistente
cmd-no-such-path = { $path }: arquivo ou diretório inexistente
cmd-no-selection = nada está selecionado
cmd-condition-false = não executado: { $command }
cmd-screen-cleared = tela limpa
cmd-stack-empty = a pilha de diretórios está vazia
cmd-stack-cleared = pilha de diretórios esvaziada
cmd-pushed = { $count } diretórios na pilha
cmd-environment = ambiente
cmd-set = { $name } = { $value }
cmd-not-set = { $name } não está definida
user-screen-title = Saída — { $lines } linhas, a partir da linha { $at }
user-screen-empty = (nada foi executado ainda)
viewer-no-branch = esta linha não tem endereço para seguir
viewer-no-back = nada para onde voltar

# ------------------------------------------------- find files (SPEC 06 §1)
folders-title = Procurar diretório
folders-reading = Lendo os diretórios…
folders-read = { $count } diretórios
folders-capped = { $count } diretórios, até o limite
folders-local-only = a lista de diretórios é desta máquina; um servidor tem o seu histórico e os atalhos
find-title = Localizar arquivo
find-mask = Máscara ou máscaras de arquivo
find-size = Tamanho (100M, 1M-1G, -4K)
find-text = Contendo o texto
find-case = Distinguir maiúsc./minúsc.
find-words = Somente palavras inteiras
find-hex = O texto acima é um padrão hexadecimal
find-not = Arquivos que NÃO o contêm
find-scope = Procurar
find-scope-panel = somente no diretório atual
find-scope-tree = a partir do diretório atual
find-scope-selection = nos diretórios selecionados
find-scope-root = a partir da raiz desta unidade
find-scope-other = a partir do outro painel
find-links = Procurar em links simbólicos
find-dirs = Procurar diretórios também
find-archives = Procurar em arquivos compactados
find-results = { $count } encontrados · { $files } arquivos e { $dirs } diretórios examinados
find-done = { $count } encontrados em { $files } arquivos · { $elapsed }
find-none = nada encontrado
find-stopped = interrompido após { $files } arquivos
find-hint = ↑↓ escolher · Enter ir para · F3 ver · F4 editar · F5 painel · Esc parar
find-hint-done = ↑↓ escolher · Enter ir para · F3 ver · F4 editar · F5 painel · Esc fechar
find-panelised = { $count } resultados no painel
find-needs-mask = é preciso uma máscara: `*` significa todos os arquivos
find-bad-size = `{ $text }` não é um tamanho — { $error }

# ------------------------------------------------- replace across files
replace-title = Substituir nos arquivos
replace-needle = Procurar por
replace-with = Substituir por
replace-case = Distinguir maiúsc./minúsc.
replace-words = Somente palavras inteiras
replace-pattern = O texto acima é uma expressão regular
replace-hex = Ambos são bytes em hexadecimal
replace-scope = { $count } arquivos selecionados
replace-needs-needle = não há nada para procurar
replace-confirm-title = Substituir
replace-confirm-body = { $count } ocorrências em { $files } arquivos seriam alteradas.
replace-nothing = nada para substituir: nenhum arquivo seria alterado
replace-done = { $count } ocorrências substituídas em { $files } arquivos

# ── archives (SPEC 04 §4) ───────────────────────────────────────────────────────────────
arc-writing-back = Gravando { $name } de volta no arquivo compactado
arc-move-out = para tirar algo de um arquivo compactado, copie com F5
arc-add-files-only = só arquivos entram um de cada vez; Shift+F1 põe um diretório inteiro

rename-title = Renomear vários
rename-name = Nome
rename-ext = Extensão
rename-search = Procurar por
rename-replace = Substituir por
rename-regex = Expressão regular ($1 na substituição)
rename-case = Letras
rename-case-same = como estão
rename-case-lower = minúsculas
rename-case-upper = MAIÚSCULAS
rename-case-title = Cada Palavra Com Maiúscula
rename-case-sentence = Só a primeira letra
rename-counter = Contador: início, passo, dígitos
rename-fields = [N] nome  [E] extensão  [N1-3] parte  [C] contador  [d] [t] data, hora  [P] diretório
rename-not-in-archive = vários arquivos são renomeados num diretório, não dentro de um arquivo compactado: lá, F6 renomeia uma entrada
rename-bad-counter = o contador são três números inteiros, início, passo, dígitos: { $counter } não é
rename-cannot = não é possível renomear: { $why }
rename-problem-empty = não sobrou nome
rename-problem-invalid = não é um nome que um arquivo possa ter
rename-problem-twice = outro arquivo também recebe este nome
rename-problem-exists = já existe um arquivo com este nome
rename-more = … e mais { $count }
rename-blocked =
    { $count ->
        [one] um arquivo não pode receber o novo nome: nada é renomeado até que possa
       *[other] { $count } arquivos não podem receber os novos nomes: nada é renomeado até que possam
    }
rename-summary = { $count } de { $total } mudam
rename-nothing = nenhum nome muda
rename-nothing-to-undo = não há renomeação em lote para desfazer
rename-done =
    { $count ->
        [one] um arquivo renomeado · desfaça pelo menu Arquivos
       *[other] { $count } arquivos renomeados · desfaça pelo menu Arquivos
    }
rename-undone =
    { $count ->
        [one] um nome restaurado
       *[other] { $count } nomes restaurados
    }
pack-title = Adicionar ao arquivo compactado
pack-name = Arquivo compactado
pack-format = Formato
pack-level = Compressão
pack-solid = Blocos sólidos (7z: menor, mais lento para ler um só arquivo)
pack-delete-after = Apagar os arquivos depois de compactar
pack-level-normal = normal
pack-level-fast = rápida
pack-level-best = máxima
pack-scope = { $count } arquivos selecionados
pack-needs-name = o arquivo compactado precisa de um nome
pack-format-read-only = { $format } aqui é só lido, não gravado: escolha outro formato ou outro nome
pack-single-one-file = um { $format } contém um único arquivo: escolha um, ou use { $tar } para um diretório ou vários arquivos
unpack-title = Descompactar
unpack-to = Descompactar para
unpack-scope = de { $name }
unpack-not-an-archive = não é um arquivo compactado que saibamos ler

# ── the special panel kinds (SPEC 02 §7) ────────────────────────────────────────────────
panel-quick-view = Visualização rápida
panel-info = Informações
panel-tree = Árvore
compare-done = diferem: { $left } à esquerda, { $right } à direita
cmp-subdirs = Entrar nos subdiretórios
cmp-depth = Até que profundidade (0 = até o fim)
cmp-trees-title = O que difere
cmp-trees-working = comparando as árvores: { $seen } arquivos, { $read } lidos
cmp-trees-done = { $differ } diferem, { $left } só à esquerda, { $right } só à direita, de { $pairs } pares
cmp-trees-same = as duas árvores são iguais ({ $pairs } pares, { $same } verificados)
menu-duplicates = Procurar duplicados
dup-title = Arquivos duplicados
dup-working = procurando duplicados: { $seen } arquivos, { $hashed } lidos
dup-done = { $groups } grupos de cópias, { $files } arquivos, { $wasted } desperdiçados
dup-none = nada aqui é cópia de outra coisa ({ $seen } arquivos)
dup-stopped = interrompido após { $seen } arquivos
dup-partial = interrompido antes do fim
cmp-title = Comparar
cmp-case = Distinguir maiúsc./minúsc. nos nomes
cmp-size = Um tamanho diferente é uma diferença
cmp-time = Uma hora diferente é uma diferença
cmp-seconds = Horas com até este número de segundos de distância são o mesmo momento
cmp-contents = Ler os bytes
cmp-when-time = Lê-los só onde as horas diferem
cmp-fragment = Ler só este número de KiB iniciais (0 = tudo)
cmp-ignore-eol = Fins de linha, sozinhos, não são diferença
cmp-selected = Somente o que está selecionado
cmp-stop = Parar na primeira diferença
cmp-newer = Selecionar só a cópia mais nova
menu-compare-setup = Comparar co&m opções
compare-contents-eol-ignored = { $count } diferem só nos fins de linha, ignorados
compare-contents-trusted = { $count } considerados iguais sem leitura
compare-contents-working = Comparando conteúdo: { $done } de { $total }
compare-contents-done = { $differ } de { $pairs } pares diferem
compare-contents-same = todos os { $pairs } pares são idênticos
compare-contents-none = nenhum arquivo tem o mesmo nome dos dois lados
compare-contents-local = comparar por conteúdo exige os dois painéis num sistema de arquivos local
compare-contents-eol = { $count } só nos fins de linha
compare-contents-only = { $count } só de um lado
compare-contents-unread = { $count } não puderam ser lidos
compare-contents-stopped = comparação interrompida
quick-view-directory = { $name } é um diretório — F3 o mede
quick-view-not-here = os bytes não estão aqui: F3 os traz primeiro
info-path = Caminho
info-kind = Tipo
info-kind-local = um diretório nesta máquina
info-kind-remote = em { $host }
info-kind-archive = dentro de { $name }
info-kind-list = uma lista de arquivos
info-device = Dispositivo
info-folder = Diretório
info-device-value = { $total }, { $free } livres, clusters de { $cluster }
info-slack = Sobra nos clusters
info-filesystem = Sistema de arquivos
info-mount = Montado
info-git = Repositório
info-git-clean = igual ao índice
info-git-differing = { $count } de { $tracked } diferem do índice
info-git-uncounted = não contado
info-git-merge = merge em andamento
info-git-rebase = rebase em andamento
info-git-cherry-pick = cherry-pick em andamento
info-git-revert = revert em andamento
info-git-bisect = bisect em andamento
info-memory = Memória
info-memory-value = { $free } livres de { $total }
info-memory-total = { $total }
info-contents = Contém
info-contents-value = { $files } arquivos, { $dirs } diretórios, { $bytes }
info-selected = Selecionado
info-selected-value = { $count } arquivos, { $bytes }
info-cursor = Cursor
info-size = Tamanho
info-modified = Modificado
info-description = Descrição
key-mark = Marcar
key-asm = Asm
key-block = Bloco
key-screens = Telas
key-site-edit = EdSess
key-site-new = NvSess
key-site-forget = ApSess
key-ai-edit = Editar
key-ai-new = Novo
key-ai-remove = Remover
key-ai-forget = Esquec
key-ai-threads = Histór
hex-needs-a-file = este buffer não tem arquivo de onde ler bytes
hex-save-first = salve primeiro: os dois editores não podem guardar as alterações não salvas um do outro

# ── the hex editor's block operations (SPEC 05 §2.6) ────────────────────────────────────
hex-block-title = Bloco
hex-op-write = Gravar os { $size } num arquivo
hex-op-read = Ler um arquivo em { $at }
hex-op-insert = Inserir bytes zero em { $at }
hex-op-copy = Copiar bytes de outro lugar para { $at }
hex-op-move = Mover bytes de outro lugar para { $at }
hex-op-fill = Preencher os { $size } com um byte
hex-op-xor = Aplicar ou-exclusivo aos { $size } com uma chave
hex-op-delete = Apagar os { $size } — { $was } passa a { $becomes }
hex-write-title = Gravar o bloco em
hex-read-title = Ler qual arquivo
hex-fill-title = Preencher com qual byte (hex)
hex-xor-title = Ou-exclusivo com qual chave (hex)
hex-insert-title = Quantos bytes zero
hex-copy-title = Copiar de onde — deslocamento e comprimento (hex)
hex-move-title = Mover de onde — deslocamento e comprimento (hex)
hex-not-hex = isso não é uma sequência de dígitos hexadecimais
hex-not-a-count = isso não é um número de bytes
hex-not-a-range = isso não é um deslocamento e um comprimento em hex
hex-range-outside = esse intervalo está fora do arquivo ({ $size })
hex-move-into-itself = um bloco não pode ser movido para dentro de si mesmo
hex-filled = { $size } preenchidos
hex-xored = { $size } com ou-exclusivo aplicado — isto não é criptografia
hex-written = { $size } gravados em { $name }
hex-read-done = { $size } lidos de { $name }
hex-inserted = { $size } de zeros inseridos
hex-deleted = { $size } apagados
hex-device = { $name }: um dispositivo de { $size }, aberto somente para leitura
asm-title = Montar em { $at } ({ $arch })
asm-no-arch = { $arch } não é uma arquitetura para a qual sabemos montar
asm-written = { $text } — { $bytes }
asm-written-longer = { $text } — { $bytes } ({ $grew } bytes a mais)
screens-title = Telas
screens-panels = Painéis
quit-title = Sair do FinalCommander
quit-body = Deseja sair?
screens-quit-title = Alterações não salvas
screens-quit-body = Estas telas têm alterações que ninguém gravou:
macro-recording = Gravando uma macro · Ctrl+. encerra
macro-empty = nada foi gravado
macro-dropped = a gravação foi descartada
macro-key-title = Qual tecla
macro-key-body = { $steps } passos gravados. Pressione a tecla a que esta macro responderá, ou Esc para descartá-la.
macro-describe = Para que serve a macro em { $key }
macro-saved = macro em { $key } salva, { $steps } passos
macro-none = nenhuma macro gravada ainda · Ctrl+. grava uma
macro-list-title = Macros
macro-play-title = Como executá-la
macro-play-once = Uma vez, de onde o cursor está
macro-play-each = Uma vez para cada arquivo selecionado
macro-sure-title = Executar esta macro
macro-sure-body = A macro em { $key } copia, move, apaga ou substitui. Executá-la?
macro-too-deep = uma macro não pode executar a si mesma para sempre
plugin-list-title = Comandos de plugin
plugin-none-title = Nenhum plugin instalado
plugin-none-open = Abrir o diretório de plugins ({ $dir })
plugin-none-help = Como os plugins funcionam, e onde encontrá-los
plugin-help-heading = Plugins
plugin-why-off = desligado em Opções → Configuração de plugins
plugin-why-panel = só nos painéis
plugin-why-viewer = só no visualizador
plugin-why-editor = só no editor
plugin-why-local = precisa de um diretório nos discos desta máquina
plugin-why-files = precisa de um arquivo sob o cursor ou selecionado
plugin-why-masks = só para { $masks }
plugin-why-bad-mask = a máscara dele está errada: { $error }
plugin-why-here = precisa de { $names } neste diretório
plugin-why-inside = precisa de { $names } aqui ou num diretório acima
plugin-cannot = { $label }: { $why }
plugin-running = { $label }…
plugin-on-selected = em { $count } arquivo(s) selecionado(s)
plugin-failed = { $id }: { $why }
plugin-unknown-command = um plugin pediu `{ $id }`, que não é um comando
hex-copied = { $size } copiados de { $from }
hex-moved = { $size } movidos de { $from }
menu-calculator = Calculadora
menu-terminal = Terminal
calc-title = Calculadora
calc-decimal = decimal
calc-hex = hex
calc-octal = octal
calc-binary = binário
calc-characters = caracteres

# The archive commands (Shift+F3) and the two parts of the order that are not the key.
arc-commands-title = Comandos de compactação
arc-password = Senha de { $name }
arc-op-test = Testar { $name }
arc-op-extract-here = Descompactar { $name } aqui
arc-op-extract-to = Descompactar { $name } para…
arc-op-delete = Apagar { $name }
arc-op-add = Adicionar arquivos a um arquivo compactado…
job-test = Testando arquivo compactado
msg-dirs-first-on = diretórios primeiro
msg-dirs-first-off = diretórios junto com os arquivos
msg-selected-first-on = arquivos selecionados primeiro
msg-selected-first-off = arquivos selecionados na ordem

# The change-drive dialog (Alt+F1, Alt+F2): the type column, and its footer.
drive-fixed = Fixo
drive-removable = Removível
drive-optical = Óptico
drive-network = Rede
drive-memory = Memória
drive-home = Pasta pessoal
drive-connection = Conexão
drive-objects = Armazenamento de objetos
drive-netbox = NetBox: sessões guardadas
drive-processes = Lista de processos
drive-temp = Painel temporário
temp-added = { $count } adicionados ao painel temporário
proc-none = a lista de processos em execução não pode ser lida aqui
drive-programs = Programas instalados
drive-services = Serviços
svc-none = esta máquina não tem um gerenciador de serviços que possamos consultar
temp-not-a-file = as linhas aqui não são arquivos: não há nada para apagar
svc-card = Serviço
svc-card-id = Nome
svc-card-name = Exibido como
svc-card-state = Estado
svc-card-description = Descrição
svc-card-pid = Processo
svc-state-running = em execução
svc-state-stopped = parado
svc-state-failed = falhou
svc-state-other = outro
key-service-start = Inicia
key-service-restart = Reinic
key-service-stop = Parar
svc-ask-stop = Parar { $name }?
svc-ask-restart = Reiniciar { $name }?
btn-service-stop = Parar
btn-service-restart = Reiniciar
svc-working = { $name }: consultando o sistema…
svc-started = { $name } iniciado
svc-stopped = { $name } parado
svc-restarted = { $name } reiniciado
svc-needs-root = { $name }: isto exige root — o programa não vai pedi-lo em seu nome
svc-failed = { $name }: { $error }
prog-none = nada aqui diz o que está instalado
prog-card = Programa instalado
prog-no-uninstall = nada diz como remover { $name }
prog-uninstall-ready = o comando que remove { $name } está na linha de comando, sem ter sido executado
prog-card-name = Nome
prog-card-version = Versão
prog-card-publisher = Fornecedor
prog-card-size = Tamanho no disco
prog-card-installed = Instalado
prog-card-location = Diretório
prog-card-uninstall = Removido por
drive-sockets = Lista de sockets
sock-none = nada está escutando aqui, ou a tabela não pode ser lida
sock-no-process = nada diz qual processo detém este socket
proc-card = Processo
proc-gone = { $name } não está mais em execução
proc-card-name = Nome
proc-card-pid = Id do processo
proc-card-parent = Iniciado por
proc-card-user = Usuário
proc-card-state = Estado
proc-card-threads = Threads
proc-card-memory = Memória
proc-card-mapped = Mapeada
proc-card-bits = Bits
proc-card-started = Iniciado
proc-card-exe = Executável
proc-card-cwd = Diretório
proc-card-command = Linha de comando
proc-card-group = Grupo
proc-card-sid = SID
proc-card-priority = Prioridade
proc-card-session = Sessão
proc-card-uptime = Em execução há
proc-card-cpu-user = Tempo de processador, próprio
proc-card-cpu-kernel = Tempo de processador, no kernel
proc-card-handles = Handles abertos
proc-card-faults = Falhas de página
proc-card-private = Memória privada
proc-card-swap = Em swap
proc-card-io-read = Lido
proc-card-io-write = Gravado
proc-card-memory-peak = Pico do conjunto de trabalho
proc-card-private-resident = Conjunto de trabalho, privado
proc-card-swap-peak = Pico do arquivo de paginação
proc-card-virtual = Bytes virtuais
proc-card-virtual-peak = Pico de bytes virtuais
proc-card-paged-pool = Pool paginado
proc-card-nonpaged-pool = Pool não paginado
proc-card-io-other = Outras E/S
proc-card-io-read-ops = Operações de leitura
proc-card-io-write-ops = Operações de gravação
proc-card-io-other-ops = Outras operações
proc-kill-title = Encerrar processo
proc-kill-ask = Encerrar { $name } [{ $pid }]? O que ele não salvou será perdido.
proc-killing = Encerrando { $name }…
proc-killed = { $name } encerrado
proc-kill-failed = { $name } não pôde ser encerrado: { $why }
proc-card-env = Ambiente
proc-card-modules = Módulos carregados
proc-card-version = Versão
proc-card-description = Descrição
drives-hint = ↑↓ escolher · 0-9 painéis · Ctrl+1..9 colunas · F9 todas · Ctrl+R re-ler
drives-title = Alterar unidade: o que mostrar
drives-col-type = o tipo
drives-col-device = o dispositivo
drives-col-label = o nome de volume
drives-col-fs = o sistema de arquivos
drives-col-size = os tamanhos
drives-col-removable = as unidades removíveis
drives-col-panels = os painéis que não são unidades
drives-col-optical = as unidades ópticas
drives-col-network = os compartilhamentos de rede
drives-shown = mostrando { $what }
drives-hidden = ocultando { $what }
drives-reread = as unidades foram relidas
bookmarks-hint = Enter ir · Ins guardar · Del remover · F4 nome · Esc fechar

# The search dialog (F7), which asks the whole question in one place.
search-title = Procurar
search-for = Procurar por
search-mode = Como
search-mode-text = texto
search-mode-hex = hex
search-mode-regex = expressão regular
search-case = Distinguir maiúsc./minúsc.
search-words = Somente palavras inteiras
btn-find-next = Próxima
btn-find-prev = Anterior
# The help: the chapters' own names, and the note on a chapter nobody has translated yet
help-chapter-about = O que é isto
help-chapter-panels = Os painéis
help-chapter-files = Arquivos: copiar, mover, procurar, arquivos compactados
help-chapter-viewer = O visualizador
help-chapter-editor = O editor, o painel hex e o montador
help-chapter-network = Servidores
help-chapter-config = Configuração, teclas e cores
help-chapter-keys = Teclas, como estão associadas agora
help-chapter-commands = Todos os comandos, e suas teclas
help-chapter-options = Todas as configurações, e seus valores
help-here-panels = Os painéis
help-here-viewer = O visualizador
help-here-editor = O editor
help-here-hex = O editor hex
help-here-dialog = Este diálogo
help-in-english = Este capítulo ainda não foi traduzido para o seu idioma, por isso segue em inglês. As teclas e as configurações que ele cita são as mesmas em todos os idiomas.
help-about-byline = FinalCommander { $version } — por Matteo Baccan
key-ok = OK
key-cancel = Cancel.
key-dialog-move = Mover
key-tree = Árvore
key-config = Config
browse-local-only = um arquivo é escolhido nesta máquina; um caminho num servidor é digitado, e Tab o completa
key-browse = Navegar
table-prompt-filter = Linhas contendo
table-sorted = classificado por { $column }
table-sorted-back = classificado por { $column }, ao contrário
table-unsorted = de volta à ordem em que o arquivo as traz
table-filtered = restam { $rows } linhas
table-unfiltered = todas as linhas de novo
table-cut-sorted = das primeiras { $rows } linhas, que é tudo o que foi lido
table-too-many = registros demais para classificar ou filtrar aqui
table-copied =
    { $rows ->
        [one] uma linha na área de transferência
       *[other] { $rows } linhas na área de transferência
    }
table-headed = a primeira linha é o cabeçalho
table-not-headed = a primeira linha são dados: as colunas são chamadas pelo número
table-named-fields = este formato nomeia os próprios campos: não há cabeçalho para adivinhar

# Splitting and combining (SPEC 03 §9)
split-title = Dividir arquivo
split-target = Em
split-size = Cada parte
split-size-hint = 1440K, 700M, 4G — a última parte é o que sobra
split-count = Ou em
split-count-hint = partes — um número aqui prevalece sobre o tamanho
split-bad-count = { $count } não é um número de partes: duas ou mais
split-local-file = Dividir e juntar funcionam com um arquivo desta máquina
split-bad-size = “{ $size }” não é um tamanho: escreva 700M ou 4G
combine-not-a-piece = Isso não é uma parte: as partes de um arquivo dividido terminam em .001, .002, … ou vêm com um .crc
pieces-working = Dividindo ou juntando: { $done } de { $total }
pieces-cancelled = Interrompido, e o que foi gravado foi desfeito
split-done = { $parts } partes gravadas, e { $list } ao lado delas
combine-done-checked = { $name } juntado a partir de { $parts } partes, { $bytes }, e confere com o seu .crc
combine-done-unchecked = { $name } juntado a partir de { $parts } partes, { $bytes } — não havia .crc para conferir
pieces-failed = Não feito: { $error }

# The branch view (SPEC 02 §3d)
branch-reading = Lendo todos os arquivos sob { $path }…
branch-title = Ramo: { $path }
branch-done = { $count } arquivos, de todos os diretórios abaixo deste · Ctrl+Alt+B volta
branch-truncated = { $count } arquivos, e a varredura parou aí: o ramo é maior do que uma lista consegue ser útil
branch-not-here = A visão de ramo é de um diretório, e este painel é uma lista

## The release check (SPEC 11 §5).
menu-check-update = Verificar atuali&zações
opt-update-check = Verificar se há nova versão uma vez por dia
btn-open-page = Abrir a página
update-checking = Consultando o GitHub sobre a versão mais recente…
update-title = Atualizações
update-available = O FinalCommander { $version } foi lançado (este é o { $current }).
update-available-status = O FinalCommander { $version } foi lançado: Opções → Verificar atualizações
update-current = Esta é a versão mais recente, { $version }.
update-failed = Não foi possível consultar a versão mais recente: { $why }

## Duplicate files (SPEC 06 §13).
menu-dupes = Procurar arquivos duplicados
dupes-title = Arquivos duplicados
dupes-recursive = Procurar nos subdiretórios
dupes-min-size = Menor arquivo
dupes-min-size-hint = Um tamanho como os filtros escrevem: 1, 100K, 5M. Arquivos vazios nunca são comparados.
dupes-select = Selecionar todas as cópias menos a mais antiga
btn-search = Procurar
dupes-not-here = A procura de duplicados parte de um diretório, ou do que está selecionado.
dupes-bad-size = Não é um tamanho: { $size }
dupes-looking = Procurando arquivos duplicados…
dupes-progress = Duplicados: passada { $pass } de 3, { $done }/{ $total }
dupes-none = Nenhum duplicado entre { $count } arquivos.
dupes-panel = Duplicados: { $groups } grupos
dupes-done = { $groups } grupos de duplicados, { $size } nas cópias.
dupes-truncated = Interrompido antes do fim: { $groups } grupos até agora, { $size } nas cópias.

## SQLite queries (SPEC 04 §4b).
sql-empty = Escreva uma consulta depois de sql:, como sql: SELECT * FROM t
sql-no-database = sql: precisa de um banco de dados SQLite: entre em um, ou ponha o cursor sobre um.
sql-running = Executando a consulta…
sql-failed = A consulta falhou: { $why }

## Asking a language model (SPEC 06 §14).
ai-none = Nenhum serviço de IA configurado: adicione um bloco [[ai]] ao fcmd.toml (name, url, model, api, token_env).
ai-title = Perguntar a uma IA
ai-no-thread = não há resposta para continuar: pergunte com `ai:` primeiro, ou ponha o cursor sobre uma no diretório de respostas
ai-nothing-to-explain = nada para explicar aqui: abra um arquivo no visualizador ou no editor, ou pergunte logo depois de um comando que falhou
ai-explain-question = O que é isto, de { $name }? Explique brevemente.
ai-tokens = { $in } tokens de entrada, { $out } de saída
ai-none-panel = Ainda não há serviço de IA — F7 adiciona um
ai-service-removed = { $name } removido
ai-probing = Entrando em { $name } em { $host }…
ai-probe-ok = { $host } respondeu HTTP { $status } em { $ms } ms, { $models } modelos — digite uma pergunta e pressione Enter
ai-probe-no-model = (o modelo { $model } não está entre eles)
ai-row-context = contexto
ai-row-conversation = conversa
ai-session-title = IA: { $name } · { $model } — { $files } arquivos no contexto
ai-context-added = { $count ->
        [one] Um arquivo
       *[other] { $count } arquivos
    } no contexto
ai-context-nothing = Nada para adicionar: só arquivos deste disco entram no contexto
ai-command-ready = o comando está na linha — leia-o antes de pressionar Enter
ai-commit-question = Escreva uma mensagem de commit para as alterações preparadas.
ai-staged-diff = o diff preparado
ai-send-diff = O diff preparado vai para { $host }
ai-send-files = { $count } arquivos, { $size }, serão enviados a { $host } com a pergunta.
ai-asking = Perguntando a { $name }…
ai-busy = { $name } está pensando… { $secs } s · Esc interrompe
ai-conversation-new = Uma nova conversa: a próxima pergunta a inicia.
ai-live-empty = Digite uma pergunta na linha de comando: ela vai para { $name }, e a conversa aparece aqui à medida que acontece.
ai-failed = A IA não respondeu: { $why }
ai-stopped = interrompido: a pergunta para { $name } foi descartada, e nada foi gravado
ai-context-title = Arquivos no contexto
ai-context-all = Todos
ai-context-dropped = { $count ->
        [one] um arquivo fora do contexto
       *[other] { $count } arquivos fora do contexto
    }
ai-context-hint = Enter ou Del: tirar do contexto — o arquivo continua no disco · F3: o que é enviado · Esc
ai-threads-title = Conversas
ai-threads-hint = Enter: continuar esta conversa · Esc
ai-thread-resumed = continuando { $name }
ai-next-estimate = próxima ≈ { $tokens }
ai-next-over = próxima ≈ { $tokens }, mais do que o modelo comporta
ai-send-estimate = Cerca de { $tokens } tokens no total, com a conversa até aqui.
ai-send-estimate-of = Cerca de { $tokens } tokens no total, com a conversa até aqui, dos { $max } que o modelo comporta.
ai-send-over = Cerca de { $tokens } tokens no total: mais que os { $max } que o modelo comporta, então o início pode ser cortado. F8 tira um arquivo.
ai-send-question = A pergunta vai para { $host }.
btn-send = Enviar

## Git (SPEC 06 §15).
menu-git = Git
git-title = Git
git-status = O que mudou
git-log = O log
git-blame = Blame de { $name }
git-branches = Ramos
git-local-only = o git trabalha num diretório deste disco.
git-running = Consultando o git…
git-failed = git: { $why }
git-clean = Nada mudou.
git-status-title = Alterados: { $staged } preparados, { $unstaged } não
git-side-conflict = conflito
git-side-staged = preparado
git-side-changed = não preparado
git-side-untracked = não rastreado
git-kind-modified = modificado
git-kind-added = novo
git-kind-deleted = apagado
git-kind-renamed = renomeado
git-kind-copied = copiado
git-kind-type = tipo alterado
git-kind-conflict = a resolver
git-as-tree = Mostrar o que mudou como árvore
git-as-list = Mostrar o que mudou como lista
git-version-head = último commit
git-version-index = preparado
git-version-disk = no disco
git-discard-title = Descartar alterações
git-discard-body = Descartar as alterações de { $count ->
        [one] um arquivo
       *[other] { $count } arquivos
    }?
git-discard-worktree = de volta ao índice:
git-discard-both = de volta ao último commit:
git-discard-added = fora do índice, mantido no disco:
git-discard-trash = para a lixeira:
git-discard-warning = O que volta ao índice ou ao último commit não pode ser desfeito.
git-hint = Enter: escolher · Esc: fechar

## The editor's character list and word completion.
charmap-title = Caracteres
charmap-hint = Digite para filtrar (um caractere ou seu código, U+00E9) · Enter: inserir · Esc: voltar
editor-no-completion = Nenhuma outra palavra no arquivo começa assim.

## A stored session's address, copied (NetBox's Generate URL).
net-url-copied = Copiado { $url }

## The system log (SPEC 06 §16).
menu-syslog = Log do sistema
syslog-reading = Lendo o log do sistema…

## Deleting lines by pattern (FAR #10).
editor-prompt-delete-matching = Remover as linhas que combinam com (! no início: manter só essas)
editor-lines-deleted = Linhas removidas: { $count }

## The copy dialog's size policy (FAR #493).
copy-conflict-size = substituir quando o tamanho for diferente

# Copy dialog: what an error does (FAR's issue #597)
copy-on-error = Se algum não puder ser copiado
copy-error-ask = Perguntar
copy-error-skip = Pular e continuar
copy-error-stop = Parar a cópia

# Attributes dialog: the times copied from one file to another (FAR's issue #241)
attr-times-copied = Horas copiadas: Ctrl+Shift+V as coloca no diálogo de outro arquivo
attr-times-none = Nada copiado ainda: Ctrl+Shift+C no diálogo de um arquivo primeiro
attr-times-pasted = Horas coladas: OK as define

# Editor: replacements kept by name and recent ones (FAR's issue #282)
replace-tpl-title = Substituições
replace-tpl-hint = Enter executa · Ins guarda uma recente com nome · Del esquece uma guardada · Esc
replace-tpl-name = Guardar esta substituição como
replace-tpl-kept = Guardada como {$name}
replace-tpl-empty = Nenhuma substituição ainda: Ctrl+F7 primeiro

# Panel settings: a view and a sort per folder (FAR's issue #389)
opt-remember-view = Cada diretório mantém a própria visualização e classificação

# Session editor: a cap on a copy's speed to or from the site
site-max-kbps = Limite de velocidade, KB/s (vazio: nenhum)

# Session editor: IMAP, read only
site-protocol-imap = IMAP (e-mail, somente leitura)
site-protocol-imaps = IMAPS (e-mail sobre TLS, somente leitura)

# The AI services as a list, like the sessions
ai-services-title = Serviços de IA
ai-services-hint = Enter: o que ai: consulta · Ins novo · F4 alterar · Del remover · Esc
ai-service-title = Serviço de IA
ai-field-name = Nome
ai-field-api = Protocolo
ai-field-url = Endereço
ai-field-model = Modelo
ai-field-token-env = Chave na variável
ai-field-token-note = Vazio: a chave é pedida uma vez por sessão e nunca é gravada.
ai-field-system = O que lhe é dito primeiro
ai-service-incomplete = Um serviço precisa de um nome e de um endereço
ai-service-default = ai: agora consulta {$name}
ai-key-prompt = A chave de {$name} (mantida até você sair)
ai-api-openai = OpenAI e compatíveis
ai-api-anthropic = Anthropic
ai-api-ollama = Ollama (nesta máquina)
menu-ai = Serviços de IA

# The secrets file
vault-password = A senha do arquivo de segredos do fcmd
vault-new = Guardar esta chave no arquivo de segredos criptografado do fcmd: escolha a senha dele (vazio: só para esta sessão)
vault-refused = O arquivo de segredos não abriu: {$why}

# The programs from elsewhere
drive-tools = Programas externos
drive-ai = IA
drive-git = Git
tools-found = encontrado
tools-missing = não instalado
tools-summary = { $found } de { $total } programas externos encontrados; F3 diz para que serve cada um, F4 instala um que falta
tools-card = Programa externo
tools-card-program = Programa
tools-card-where = Onde
tools-card-version = Versão
tools-card-for = Usado para
tools-card-install = Para instalar
tools-card-why = Por que o fcmd o executa em vez de fazer ele mesmo: F1, “Programas de fora”.
tools-install-any = instale o pacote que fornece { $name } e confira que ele está no PATH
tools-already = { $name } já está aqui
tools-for-archives = arquivos compactados: { $what }
tools-for-decompile = Shift+F6 no visualizador em { $what }
tools-for-git = o painel Git (Ctrl+Alt+G) e o ramo no prompt
tools-for-video = os quadros de um vídeo no visualizador e na visualização rápida (F3, Ctrl+Q)
tools-for-open = abrir um link no navegador
tools-for-syslog = Comandos → Log do sistema
tools-for-services = o painel de serviços
tools-card-site = Site
tools-no-package = Não se conhece gerenciador de pacotes nesta máquina que tenha { $name }. Abrir a página de download?
tools-install-ask = Instalar { $name } executando, num terminal:
tools-install-how = É no terminal que o sudo pede a sua senha. Ctrl+Alt+E volta aos painéis, e Ctrl+R neste painel verifica de novo.
tools-install-else = Se falhar: { $url }
tools-install-failed = fcmd: { $name } não foi instalado. Baixe-o de { $url }
tools-installing = instalando { $name } num terminal: Ctrl+Alt+E volta aos painéis, Ctrl+R lá verifica de novo
btn-install = Instalar

# Plugins configuration
plugins-config-title = Plugins
plugins-config-hint = Enter: ligar ou desligar · F3 o que é e o que traz · Esc
plugins-broken = não carregou — F3 diz por quê
plugins-cannot-toggle = um plugin que não carregou não pode ser ligado: F3 diz o que há de errado com ele
plugins-on = { $id } está ligado: seus comandos já estão em Ctrl+Shift+F11
plugins-on-restart = { $id } está ligado: seus comandos já estão em Ctrl+Shift+F11, o resto que ele traz vem na próxima inicialização
plugins-off = { $id } está desligado: seus comandos saíram de Ctrl+Shift+F11
plugins-off-restart = { $id } está desligado: seus comandos saíram de Ctrl+Shift+F11, o resto que ele traz sai na próxima inicialização
plugins-state-on = ligado
plugins-state-off = desligado
plugins-card-id = Id
plugins-card-name = Nome
plugins-card-version = Versão
plugins-card-author = Autor
plugins-card-about = Sobre
plugins-card-dir = Diretório
plugins-card-state = Estado
plugins-card-brings = Traz
plugins-card-commands = Comandos (Ctrl+Shift+F11):
plugins-card-notes = Dito ao carregar:
plugins-none = nenhum plugin encontrado · eles ficam em { $dir }, um diretório para cada, com um plugin.toml

# The git panel
git-file-log = O log de { $name }
git-commit = Fazer commit do que está preparado…
git-pull = Pull
git-push = Push
git-pushing = Fazendo push…
git-pulling = Fazendo pull…
git-commit-prompt = A mensagem do commit
git-commit-empty = Um commit precisa de uma mensagem.
git-already-on = { $branch } é o ramo em que você está.
git-switch-ask = Mudar para { $branch }? Os arquivos em que os dois ramos diferem mudam no disco.
git-done = git: { $said }
git-branches-title = Ramos: { $count }
git-log-title = Log: { $count }
git-log-of-title = Log de { $of }: { $count }
btn-switch = Mudar
key-git = Git
key-git-diff = Diff
key-git-stage = Prepar
key-git-unstage = Retira
key-git-discard = Descart
key-git-commit = Commit
key-git-show = Mostra
key-git-log = Log
key-git-changes = Alter.
key-git-pull = Pull
key-git-push = Push
key-git-branches = Ramos

undo-title = Desfazer
btn-undo = Desfazer
menu-undo = Desfazer a última operação
menu-strip-meta = Remover metadados de imagens…
undo-nothing = nada para desfazer: nenhuma cópia, movimentação, exclusão para a lixeira ou novo diretório desde que o programa iniciou
undo-copy =
    { $count ->
        [one] Enviar para a lixeira o arquivo que a cópia criou:
       *[other] Enviar para a lixeira os { $count } arquivos que a cópia criou:
    }
undo-move =
    { $count ->
        [one] Mover o arquivo de volta para onde estava:
       *[other] Mover os { $count } arquivos de volta para onde estavam:
    }
undo-trash =
    { $count ->
        [one] Restaurar o arquivo da lixeira:
       *[other] Restaurar os { $count } arquivos da lixeira:
    }
undo-mkdir = Remover o novo diretório, se ainda estiver vazio:
strip-title = { $count ->
        [one] Metadados de uma imagem
       *[other] Metadados de { $count } imagens
    }
strip-all = Tudo — texto, EXIF, XMP, IPTC, comentários, C2PA
strip-ai = Só o que um modelo de imagem escreveu — prompt, workflow, configurações
strip-camera = Tudo menos o EXIF da câmera e o copyright — sem GPS
strip-hint = Enter: remover; a imagem não é recodificada, Ctrl+Alt+Z desfaz · Esc
strip-nothing = nenhum PNG, WebP ou JPEG deste disco selecionado ou sob o cursor
strip-done = { $count ->
        [one] uma imagem
       *[other] { $count } imagens
    } sem metadados ({ $blocks } blocos removidos), { $clean } não tinham nada a remover · Ctrl+Alt+Z os devolve
undo-restore = { $count ->
        [one] Devolver a imagem com seus metadados:
       *[other] Devolver as { $count } imagens com seus metadados:
    }
undo-done =
    { $count ->
        [one] um nome de volta onde estava
       *[other] { $count } nomes de volta onde estavam
    }
slow-drives = a lista de unidades levou { $secs } s
slow-drives-late = a lista de unidades esperou { $secs } s: { $drives } não responderam a tempo e aparecem sem detalhes — uma unidade de rede cujo servidor não está lá?
slow-listing = a leitura de { $what } levou { $secs } s
# editor search and undo (0.41)
btn-find-all = Todas as ocorrências
btn-replace-all = Substituir todas
btn-replace-one = Substituir próxima
editor-replace-title = Substituir
editor-replace-style = Preservar estilo (foo → bar, Foo → Bar, FOO → BAR)
editor-replace-groups = Numa expressão regular, $1 e { "${name}" } são o que um grupo capturou, $$ é um cifrão
editor-matches-title = Linhas com `{ $query }`: { $count }
editor-matches-hint = Digite para filtrar · Enter: ir para a linha · Esc: voltar


# sorting and extended attributes (0.41)
opt-sort-case = Maiúsculas e minúsculas na classificação
opt-sort-case-auto = como este sistema faz
opt-sort-case-yes = distingui-las
opt-sort-case-no = um só alfabeto
copy-keep-xattrs = Preservar os atributos estendidos

# editor conversions, disk diff and guides (0.41)
editor-convert-title = Gravar o arquivo como
editor-convert-hint = Digite para filtrar · Enter: converter (F2 grava, Ctrl+Z desfaz) · Esc: voltar
editor-convert-eol = Fim de linha: { $eol }
editor-convert-bom-on = Marca de ordem de bytes (BOM): gravada
editor-convert-bom-off = Marca de ordem de bytes (BOM): não gravada
editor-convert-page = Página de código: { $page }
editor-converted-eol = { $name }: as linhas terminarão com { $what } no próximo salvamento
editor-converted-bom-on = { $name }: o próximo salvamento grava uma marca de ordem de bytes
editor-converted-bom-off = { $name }: o próximo salvamento não grava marca de ordem de bytes
editor-converted-page = { $name }: o próximo salvamento grava { $what }
editor-convert-same = O arquivo já está gravado assim
editor-convert-no-bom = { $page } não tem marca de ordem de bytes: só UTF-8 e UTF-16 têm
editor-convert-lossy = { $page } não comporta todos os caracteres de { $name }: nada foi alterado
btn-reload = Recarregar
btn-what-changed = O que mudou
editor-diff-buffer = no editor
editor-diff-hint = Esta comparação só mostra o que mudou — Esc volta à pergunta


# job journal and saved syncs (0.41)
journal-left = Cópias deixadas inacabadas pela última sessão: { $count } · Alt+Shift+F6 lista
journal-title = Operações inacabadas
journal-hint = Enter continuar · Del descartar · Esc fechar
journal-none = Nenhuma operação ficou inacabada
journal-row = { $kind }: { $what } → { $target } · arquivos concluídos: { $done }
journal-row-unreadable = não pode ser lido: { $file } ({ $error })
journal-unreadable = { $file }: não é um diário de operação que esta versão saiba ler ({ $error }). Alt+Shift+F6 o lista, e Del lá o remove
journal-resumed = Continuando { $kind }: { $done } arquivos já estavam concluídos
journal-discarded = A operação inacabada foi descartada, junto com o que ela tinha gravado pela metade
journal-cannot-resume = Esse diário não pode ser lido; Del o remove
menu-jobs-left = Operações inacabadas
sync-report = Gravar um relatório da execução em (um arquivo)
sync-name = Salvar este par como (um nome)
sync-saved-button = Salvas…
sync-saved-title = Sincronizações salvas
sync-saved-hint = Enter executar · F4 alterar · Del esquecer · Esc fechar
sync-saved-none = Nenhuma sincronização salva ainda: dê um nome a uma no diálogo (Ctrl+Shift+Y)
sync-saved-removed = { $name } não está mais salva
sync-report-local = O relatório tem de ser um arquivo desta máquina
sync-report-written = O relatório da sincronização está em { $path }
sync-report-not-written = Não foi possível gravar o relatório em { $path }: { $error }
sync-report-title = Relatório de sincronização
sync-report-name = Nome: { $name }
sync-report-left = Esquerda: { $path }
sync-report-right = Direita: { $path }
sync-report-to-right = Copiados da esquerda → direita: { $count }
sync-report-to-left = Copiados da direita → esquerda: { $count }
sync-report-cut-right = Removidos à direita: { $count }
sync-report-cut-left = Removidos à esquerda: { $count }
sync-report-skipped = Deixados como estavam, embora difiram: { $count }
sync-report-failed = Falharam: { $count }
sync-report-stopped = Interrompido no meio, então estes podem ou não ter sido feitos: { $count }
sync-report-not-done = Não feitos, porque um passo anterior falhou ou foi interrompido: { $count }
menu-sync-saved = Sincronizações salvas

# hex editor search
hex-search-wrapped = `{ $query }`: encontrado recomeçando pela outra ponta do arquivo
hex-replace-title = Substituir
hex-replace-with = Substituir por
btn-replace = Substituir
hex-replace-ask = Substituir { $was } em { $at } por { $with }?
hex-replace-length = a substituição tem { $with } bytes e a ocorrência { $was }: isso deslocaria tudo o que vem depois — pressione Ins para o modo de inserção primeiro

# pixels and video (0.41)
opt-viewer-pixels = Imagens
opt-viewer-pixels-auto = pixels reais onde o terminal os desenha
opt-viewer-pixels-kitty = protocolo gráfico do kitty
opt-viewer-pixels-iterm = imagens embutidas do iTerm2
opt-viewer-pixels-sixel = sixel
opt-viewer-pixels-off = sempre meios-blocos
video-needs-program = isto é um vídeo ({ $what }): seus quadros são desenhados através de `{ $program }`, que não está instalado nesta máquina — Shift+F11 ainda lê o contêiner
video-no-frame = nenhum quadro deste vídeo: { $why }
video-playing = reproduzindo a { $fps } quadros por segundo, sem som — Space pausa
video-paused = em pausa em { $at } — Space continua, PgUp e PgDn avançam um décimo
video-ended = o vídeo terminou — Space o reproduz de novo


# finding things
menu-saved-searches = Procuras salvas
drive-searches = Procuras salvas
find-keys = F2 guarda esta procura com um nome · F9 as guardadas
find-save-name = Guardar esta procura como
find-saved = procura guardada como { $name }
find-saved-none = nenhuma procura foi guardada ainda: F2 no diálogo de procura guarda uma
find-saved-title = Procuras salvas — Enter abre, F4 edita, Del esquece
find-saved-running = procurando { $name }…
find-saved-gone = procura { $name } esquecida
find-loaded = procura { $name } carregada
find-fuzzy = Nome aproximado: as letras acima, em ordem, melhor resultado primeiro
find-meta = Metadados (artist=bowie; camera=)
find-index = Consultar o índice do sistema em vez de varrer (pode não ter arquivos novos)
tools-for-index = o índice que Alt+F7 pode consultar em vez de varrer
menu-frecent = Diretórios frequentes
frecent-none = nenhum diretório foi visitado ainda
frecent-gone = { $path } retirado dos diretórios frequentes
frecent-hint = digite letras do caminho · Enter ir para lá · Del esquecer · Esc fechar

# ai third round
ai-redact-off = O mascaramento está desligado: tudo vai como está
ai-redact-none = Nenhum segredo encontrado para mascarar
ai-redact-count = Segredos mascarados antes do envio: { $count } ({ $kinds })
ai-redacted = segredos mascarados: { $count }
ai-preview-title = O que vai para { $name } ({ $host })
ai-preview-instruction = Instrução
ai-preview-earlier = A conversa até aqui
ai-preview-question = A pergunta e seus arquivos
ai-preview-key = F3 mostra o que é enviado
ai-preview-reading = Lendo o que seria enviado…
key-ai-redact = Mascara
ai-redact-now-on = O mascaramento está ligado: chaves, tokens, chaves privadas e senhas são mascarados antes de qualquer envio
ai-redact-now-off = O mascaramento está desligado: o que é enviado vai como está
ai-panel-redact-on = segredos mascarados
ai-panel-redact-off = segredos não mascarados
key-ai-local = Local
ai-row-refused = não é esta máquina: recusado
ai-panel-local-only = somente local
ai-local-now-on = Somente local está ligado: um serviço que não está nesta máquina é recusado, e nada é enviado a ele
ai-local-now-off = Somente local está desligado: qualquer serviço da lista pode ser consultado
ai-local-refused = Somente local está ligado: { $name } responde em { $host }, que não é esta máquina — nada foi enviado (F6 no painel de IA desliga isso)
ai-rename-prompt = O que os { $count } nomes devem virar? (em palavras)
ai-send-names = A instrução e { $count } dos { $total } nomes — só os nomes, nada dos arquivos — vão para { $host }.
ai-rename-ready = o padrão do modelo está no diálogo: leia a prévia, corrija o que estiver errado, e só OK renomeia
ai-rename-waiting = o padrão de renomeação do modelo está pronto: Ctrl+Shift+F6 abre o diálogo com ele
ai-rename-unreadable = o modelo não respondeu com um padrão de renomeação: a resposta está no diretório de respostas (ai:), e nada foi renomeado
ai-rewrite-prompt = O que deve ser feito do bloco marcado? (em palavras)
ai-rewrite-no-block = marque um bloco primeiro (Shift+setas): a reescrita substitui exatamente o que está marcado, e um bloco em coluna não tem forma para a qual voltar
ai-rewrite-read-only = este arquivo é somente leitura aqui: uma reescrita não teria para onde ir
ai-rewrite-before = o bloco
ai-rewrite-after = como o modelo o reescreveu
ai-rewrite-hint = A reescrita do modelo, frente ao bloco — Enter a põe no lugar do bloco, Esc deixa o bloco como está
diff-hint-rewrite = Enter substituir o bloco · Esc deixar
ai-rewrite-applied = o bloco foi substituído pela reescrita do modelo — um Ctrl+Z o devolve
ai-rewrite-left = o bloco ficou como estava
ai-rewrite-stale = o bloco não é mais o que era quando a reescrita foi pedida: nada foi substituído — a versão do modelo está no diretório de respostas (ai:)
ai-rewrite-same = o modelo devolveu o bloco como está: nada para substituir
ai-answer-kept = a resposta é { $name } no diretório de respostas (ai:) — o editor ficou na frente
ai-failure-question = Este comando terminou com código de saída { $code }: `{ $command }` — por que falhou, e o que o corrigiria? Brevemente.
ai-send-failure = O comando que falhou e as últimas { $lines } linhas que ele imprimiu vão para { $host }.

# archives and jobs
arc-test-whole = { $name }: { $files } entradas testadas, { $bytes } — todas íntegras
arc-test-bad-title = { $name }: { $bad } de { $files } entradas não saíram íntegras

# extending it
plugin-column-failed = { $column }: { $error } — a coluna dele fica vazia para os arquivos pelos quais não respondeu
plugins-card-columns = Colunas (como um [[column_set]] as nomeia):
usermenu-far-row = O que foi lido de forma diferente do FarMenu.ini ({ $count })
usermenu-far-notes = o menu vem do FarMenu.ini do FAR: { $count } coisas foram lidas de forma diferente — a última linha as lista
usermenu-far-not-run = Nada foi executado.
find-age = Modificado, há quantos dias (7, >=30, 7-30)
find-bad-age = `{ $text }` não é uma idade em dias — { $error }
find-label-age = { $days } d
find-keys-ai = Ctrl+Alt+Q diz a procura em palavras: um modelo de linguagem preenche estas linhas
ai-find-prompt = O que você está procurando? (em palavras)
ai-send-words = Estas palavras — e nada dos painéis: nenhum nome, caminho ou arquivo — vão para { $host }.
ai-find-ready = a procura do modelo está no diálogo: leia as linhas, corrija o que estiver errado, e só OK procura
ai-find-waiting = a procura do modelo está pronta: Alt+F7 abre o diálogo com ela
ai-find-unreadable = o modelo não respondeu com uma procura: a resposta está no diretório de respostas (ai:), e nada foi procurado
ai-find-left-out = { $said } — NÃO está nas linhas, porque elas não conseguem dizer isso: { $what }
# treemap and compare
usage-map-title = Uso do disco, a árvore inteira: { $path }
usage-map-walking = Medindo… arquivos: { $files }, { $size } até agora
usage-map-where = { $path } · { $size } · arquivos: { $files }
usage-map-unreadable = diretórios não lidos: { $count }
usage-map-rest = arquivos menores: { $count }
usage-map-others = outros: { $count }
usage-map-empty = Nada neste diretório ocupa espaço
usage-map-chosen = { $name } · { $size } · { $share }%
usage-map-hint = ←↑↓→ · Enter entrar · BkSp subir · Tab lista · G painel lá · Esc
usage-map-hint-list = ↑↓ · Enter entrar · BkSp subir · Tab mapa · G painel lá · Esc
usage-map-hint-walking = Esc interrompe
usage-map-top = Este é o diretório que foi medido: não há nada acima dele aqui
usage-map-no-file = Esta linha são muitos arquivos pequenos, não um ao qual o painel possa ir
bin-comparing = Comparando os dois arquivos byte a byte… Esc interrompe
bin-bytes = { $count } bytes
bin-same = Os dois arquivos são iguais, byte a byte: { $bytes } bytes cada
bin-where = diferença { $at } de { $count } · primeira no deslocamento { $offset } · bytes que diferem: { $differing }
bin-hint = Alt+↑↓ diferenças · ↑↓ PgUp PgDn Home End linhas · Esc fechar
diff-unified-on = Uma coluna: o que saiu, depois o que entrou
diff-unified-off = Duas colunas, lado a lado
diff-patch-title = Salvar as diferenças como patch (.diff)
diff-patch-saved = { $name } gravado: um diff unificado de { $lines } linhas
diff-patch-nothing = Os dois arquivos são iguais: não há patch para salvar
# wastebasket and external editor
drive-trash = Lixeira
drive-trash-closed = Lixeira (este sistema não deixa um programa listá-la)
menu-trash = Lixeira
menu-trash-empty = Esvaziar a lixeira
key-trash-restore = Restau.
key-trash-empty = Esvaz.
btn-empty = Esvaziar
btn-rename = Renomear
trash-closed = a lixeira deste sistema só pode ser aberta pelo gerenciador de arquivos dele
trash-empty = a lixeira está vazia
trash-failed = a lixeira: { $why }
trash-restored =
    { $count ->
        [one] um item de volta onde estava
       *[other] { $count } itens de volta onde estavam
    }
trash-purged =
    { $count ->
        [one] um item apagado definitivamente
       *[other] { $count } itens apagados definitivamente
    }
trash-card-name = Nome
trash-card-from = Estava em
trash-card-deleted = Apagado
trash-card-size = Tamanho
trash-clash-title = Restaurar
trash-clash-body = { $path } já existe.
trash-clash-note = Substituir envia o que está lá para a lixeira; Renomear põe este de volta ao lado dele.
trash-clash-more =
    { $count ->
        [one] Mais um item está na mesma situação.
       *[other] Mais { $count } itens estão na mesma situação.
    }
trash-purge-title = Apagar definitivamente
trash-purge-body =
    { $count ->
        [one] Deseja apagar { $name } da lixeira?
       *[other] Deseja apagar { $count } itens da lixeira?
    }
trash-purge-note = Isto apaga definitivamente: nada pode trazê-los de volta.
trash-empty-title = Esvaziar a lixeira
trash-empty-body = Apagar tudo o que está na lixeira?
opt-editor-external = Editor externo
opt-editor-use-external = Usar o editor externo para F4 (Alt+F4 passa a ser o interno)
opt-viewer-external = Visualizador externo
opt-viewer-use-external = Usar o visualizador externo para F3 (Alt+F3 passa a ser o interno)


# editor features
opt-editor-line-numbers = Numerar as linhas
editor-line-numbers-on = Números de linha ligados
editor-line-numbers-off = Números de linha desligados
editor-prompt-pipe-block = Passar o bloco pelo comando:
editor-prompt-pipe-file = Passar o arquivo inteiro pelo comando:
editor-prompt-insert-output = Inserir o que este comando imprime:
editor-prompt-insert-file = Inserir o arquivo:
editor-prompt-write-block = Gravar o bloco no arquivo:
editor-pipe-running = Executando { $command } …
editor-pipe-done = { $command }: substituído pelo que ele imprimiu, linhas: { $lines } (Ctrl+Z desfaz)
editor-pipe-inserted = { $command }: linhas inseridas: { $lines }
editor-pipe-empty = { $command } não imprimiu nada: nada foi inserido
editor-pipe-failed = { $command }: { $error } — nada foi alterado
editor-pipe-changed = { $command }: o texto mudou enquanto o comando rodava — nada foi substituído
editor-pipe-gone = { $command }: o arquivo para o qual ele rodou não está mais aberto — nada foi alterado
editor-pipe-column = Um bloco em coluna não pode ser passado por um comando: marque linhas inteiras, ou nada para o arquivo inteiro
editor-file-inserted = { $name } inserido, linhas: { $lines }
editor-insert-not-local = Só um arquivo desta máquina pode ser lido para o editor ou gravado a partir dele
editor-insert-too-big = grande demais para ler no editor
editor-block-written = Bloco gravado em { $name }: { $size }
editor-block-write-self = { $name } é o arquivo em edição: o bloco não foi gravado por cima dele
editor-choice-hint = Digite para filtrar · Enter: escolher · Esc: voltar
editor-syntax-title = Colorir o arquivo como
editor-syntax-auto = O que o nome dele diz: { $language }
editor-syntax-auto-none = O que o nome dele diz: nada conhecido
editor-syntax-none = Texto puro, sem cores
editor-syntax-set = { $name } agora é lido como { $language }
editor-syntax-off = { $name } agora é lido como texto puro
editor-case-heading = Alterar maiúsculas/minúsculas
editor-case-upper = MAIÚSCULAS
editor-case-lower = minúsculas
editor-case-title = Iniciais Maiúsculas: uma maiúscula no início de cada palavra
editor-case-toggle = iNVERTER: maiúsculas viram minúsculas e minúsculas viram maiúsculas
editor-no-word = Nada está selecionado e o cursor não está numa palavra
editor-prompt-isearch = Procurar ao digitar (↓ ↑ próxima e anterior, Esc volta)
editor-word-only = A palavra sob o cursor não aparece em nenhum outro lugar
editor-not-on-word = O cursor não está numa palavra
editor-prompt-reflow = Reformatar o parágrafo em quantas colunas? (só Enter: { $width })
editor-reflowed = Parágrafo reformatado em { $width } colunas (Ctrl+Z desfaz)
editor-reflow-bad-width = { $width } não é uma largura: um número de colunas de 8 a 1000
editor-no-paragraph = O cursor está numa linha vazia: não há parágrafo aqui
editor-no-name-yet = Este arquivo ainda não tem nome: salve-o primeiro
editor-tools-title = Comandos do editor
editor-tool-isearch = Procurar ao digitar
editor-tool-word-next = Próximo lugar em que a palavra sob o cursor aparece
editor-tool-word-prev = Lugar anterior em que a palavra sob o cursor aparece
editor-tool-pipe = Passar o bloco, ou o arquivo inteiro, por um comando
editor-tool-write-block = Gravar o bloco num arquivo
editor-tool-case = Alterar maiúsculas/minúsculas do bloco
editor-tool-reflow = Reformatar o parágrafo numa largura
editor-tool-insert-file = Inserir um arquivo no cursor
editor-tool-insert-output = Inserir o que um comando imprime
editor-tool-insert-date = Inserir a data e a hora
editor-tool-insert-name = Inserir o nome deste arquivo
editor-tool-insert-path = Inserir o caminho completo deste arquivo
editor-tool-line-numbers = Números de linha ligados ou desligados
editor-tool-syntax = Escolher a linguagem com que o arquivo é colorido
