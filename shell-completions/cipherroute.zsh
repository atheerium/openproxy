#compdef cipherroute

autoload -U is-at-least

_cipherroute() {
    typeset -A opt_args
    typeset -a _arguments_options
    local ret=1

    if is-at-least 5.2; then
        _arguments_options=(-s -S -C)
    else
        _arguments_options=(-s -C)
    fi

    local context curcontext="$curcontext" state line
    _arguments "${_arguments_options[@]}" : \
'--host=[]:HOST:_default' \
'--port=[]:PORT:_default' \
'--log-filter=[]:LOG_FILTER:_default' \
'--data-dir=[]:DATA_DIR:_files' \
'-h[Print help]' \
'--help[Print help]' \
":: :_cipherroute_commands" \
"*::: :->cipherroute" \
&& ret=0
    case $state in
    (cipherroute)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cipherroute-command-$line[1]:"
        case $line[1] in
            (provider)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
":: :_cipherroute__subcmd__provider_commands" \
"*::: :->provider" \
&& ret=0

    case $state in
    (provider)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cipherroute-provider-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(add)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'-h[Print help]' \
'--help[Print help]' \
':name:_default' \
':config:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_cipherroute__subcmd__provider__subcmd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cipherroute-provider-help-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(add)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(key)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
":: :_cipherroute__subcmd__key_commands" \
"*::: :->key" \
&& ret=0

    case $state in
    (key)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cipherroute-key-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(add)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'-h[Print help]' \
'--help[Print help]' \
':name:_default' \
':key:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_cipherroute__subcmd__key__subcmd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cipherroute-key-help-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(add)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(pool)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
":: :_cipherroute__subcmd__pool_commands" \
"*::: :->pool" \
&& ret=0

    case $state in
    (pool)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cipherroute-pool-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'-h[Print help]' \
'--help[Print help]' \
':name:_default' \
&& ret=0
;;
(create)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'-h[Print help]' \
'--help[Print help]' \
':name:_default' \
':proxy_url:_default' \
&& ret=0
;;
(delete)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'-h[Print help]' \
'--help[Print help]' \
':name:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_cipherroute__subcmd__pool__subcmd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cipherroute-pool-help-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(create)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(delete)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(tunnel)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
":: :_cipherroute__subcmd__tunnel_commands" \
"*::: :->tunnel" \
&& ret=0

    case $state in
    (tunnel)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cipherroute-tunnel-command-$line[1]:"
        case $line[1] in
            (start)
_arguments "${_arguments_options[@]}" : \
'--provider=[]:PROVIDER:_default' \
'--port=[]:PORT:_default' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(stop)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_cipherroute__subcmd__tunnel__subcmd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cipherroute-tunnel-help-command-$line[1]:"
        case $line[1] in
            (start)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(stop)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(route)
_arguments "${_arguments_options[@]}" : \
'--model=[Model ID (e.g. openai/gpt-4o-mini)]:MODEL:_default' \
'--combo=[Combo name]:COMBO:_default' \
'--prompt=[Prompt text]:PROMPT:_default' \
'--stream[Stream output]' \
'--json[JSON output]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(completion)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
':shell:(bash elvish fish powershell zsh)' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_cipherroute__subcmd__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cipherroute-help-command-$line[1]:"
        case $line[1] in
            (provider)
_arguments "${_arguments_options[@]}" : \
":: :_cipherroute__subcmd__help__subcmd__provider_commands" \
"*::: :->provider" \
&& ret=0

    case $state in
    (provider)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cipherroute-help-provider-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(add)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(key)
_arguments "${_arguments_options[@]}" : \
":: :_cipherroute__subcmd__help__subcmd__key_commands" \
"*::: :->key" \
&& ret=0

    case $state in
    (key)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cipherroute-help-key-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(add)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(pool)
_arguments "${_arguments_options[@]}" : \
":: :_cipherroute__subcmd__help__subcmd__pool_commands" \
"*::: :->pool" \
&& ret=0

    case $state in
    (pool)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cipherroute-help-pool-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(create)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(delete)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(tunnel)
_arguments "${_arguments_options[@]}" : \
":: :_cipherroute__subcmd__help__subcmd__tunnel_commands" \
"*::: :->tunnel" \
&& ret=0

    case $state in
    (tunnel)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cipherroute-help-tunnel-command-$line[1]:"
        case $line[1] in
            (start)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(stop)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(route)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(completion)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
}

(( $+functions[_cipherroute_commands] )) ||
_cipherroute_commands() {
    local commands; commands=(
'provider:' \
'key:' \
'pool:' \
'tunnel:' \
'route:' \
'completion:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cipherroute commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__completion_commands] )) ||
_cipherroute__subcmd__completion_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute completion commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__help_commands] )) ||
_cipherroute__subcmd__help_commands() {
    local commands; commands=(
'provider:' \
'key:' \
'pool:' \
'tunnel:' \
'route:' \
'completion:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cipherroute help commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__help__subcmd__completion_commands] )) ||
_cipherroute__subcmd__help__subcmd__completion_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute help completion commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__help__subcmd__help_commands] )) ||
_cipherroute__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute help help commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__help__subcmd__key_commands] )) ||
_cipherroute__subcmd__help__subcmd__key_commands() {
    local commands; commands=(
'list:' \
'add:' \
    )
    _describe -t commands 'cipherroute help key commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__help__subcmd__key__subcmd__add_commands] )) ||
_cipherroute__subcmd__help__subcmd__key__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute help key add commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__help__subcmd__key__subcmd__list_commands] )) ||
_cipherroute__subcmd__help__subcmd__key__subcmd__list_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute help key list commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__help__subcmd__pool_commands] )) ||
_cipherroute__subcmd__help__subcmd__pool_commands() {
    local commands; commands=(
'list:' \
'status:' \
'create:' \
'delete:' \
    )
    _describe -t commands 'cipherroute help pool commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__help__subcmd__pool__subcmd__create_commands] )) ||
_cipherroute__subcmd__help__subcmd__pool__subcmd__create_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute help pool create commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__help__subcmd__pool__subcmd__delete_commands] )) ||
_cipherroute__subcmd__help__subcmd__pool__subcmd__delete_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute help pool delete commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__help__subcmd__pool__subcmd__list_commands] )) ||
_cipherroute__subcmd__help__subcmd__pool__subcmd__list_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute help pool list commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__help__subcmd__pool__subcmd__status_commands] )) ||
_cipherroute__subcmd__help__subcmd__pool__subcmd__status_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute help pool status commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__help__subcmd__provider_commands] )) ||
_cipherroute__subcmd__help__subcmd__provider_commands() {
    local commands; commands=(
'list:' \
'add:' \
    )
    _describe -t commands 'cipherroute help provider commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__help__subcmd__provider__subcmd__add_commands] )) ||
_cipherroute__subcmd__help__subcmd__provider__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute help provider add commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__help__subcmd__provider__subcmd__list_commands] )) ||
_cipherroute__subcmd__help__subcmd__provider__subcmd__list_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute help provider list commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__help__subcmd__route_commands] )) ||
_cipherroute__subcmd__help__subcmd__route_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute help route commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__help__subcmd__tunnel_commands] )) ||
_cipherroute__subcmd__help__subcmd__tunnel_commands() {
    local commands; commands=(
'start:' \
'stop:' \
'status:' \
    )
    _describe -t commands 'cipherroute help tunnel commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__help__subcmd__tunnel__subcmd__start_commands] )) ||
_cipherroute__subcmd__help__subcmd__tunnel__subcmd__start_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute help tunnel start commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__help__subcmd__tunnel__subcmd__status_commands] )) ||
_cipherroute__subcmd__help__subcmd__tunnel__subcmd__status_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute help tunnel status commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__help__subcmd__tunnel__subcmd__stop_commands] )) ||
_cipherroute__subcmd__help__subcmd__tunnel__subcmd__stop_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute help tunnel stop commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__key_commands] )) ||
_cipherroute__subcmd__key_commands() {
    local commands; commands=(
'list:' \
'add:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cipherroute key commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__key__subcmd__add_commands] )) ||
_cipherroute__subcmd__key__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute key add commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__key__subcmd__help_commands] )) ||
_cipherroute__subcmd__key__subcmd__help_commands() {
    local commands; commands=(
'list:' \
'add:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cipherroute key help commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__key__subcmd__help__subcmd__add_commands] )) ||
_cipherroute__subcmd__key__subcmd__help__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute key help add commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__key__subcmd__help__subcmd__help_commands] )) ||
_cipherroute__subcmd__key__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute key help help commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__key__subcmd__help__subcmd__list_commands] )) ||
_cipherroute__subcmd__key__subcmd__help__subcmd__list_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute key help list commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__key__subcmd__list_commands] )) ||
_cipherroute__subcmd__key__subcmd__list_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute key list commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__pool_commands] )) ||
_cipherroute__subcmd__pool_commands() {
    local commands; commands=(
'list:' \
'status:' \
'create:' \
'delete:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cipherroute pool commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__pool__subcmd__create_commands] )) ||
_cipherroute__subcmd__pool__subcmd__create_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute pool create commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__pool__subcmd__delete_commands] )) ||
_cipherroute__subcmd__pool__subcmd__delete_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute pool delete commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__pool__subcmd__help_commands] )) ||
_cipherroute__subcmd__pool__subcmd__help_commands() {
    local commands; commands=(
'list:' \
'status:' \
'create:' \
'delete:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cipherroute pool help commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__pool__subcmd__help__subcmd__create_commands] )) ||
_cipherroute__subcmd__pool__subcmd__help__subcmd__create_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute pool help create commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__pool__subcmd__help__subcmd__delete_commands] )) ||
_cipherroute__subcmd__pool__subcmd__help__subcmd__delete_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute pool help delete commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__pool__subcmd__help__subcmd__help_commands] )) ||
_cipherroute__subcmd__pool__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute pool help help commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__pool__subcmd__help__subcmd__list_commands] )) ||
_cipherroute__subcmd__pool__subcmd__help__subcmd__list_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute pool help list commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__pool__subcmd__help__subcmd__status_commands] )) ||
_cipherroute__subcmd__pool__subcmd__help__subcmd__status_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute pool help status commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__pool__subcmd__list_commands] )) ||
_cipherroute__subcmd__pool__subcmd__list_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute pool list commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__pool__subcmd__status_commands] )) ||
_cipherroute__subcmd__pool__subcmd__status_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute pool status commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__provider_commands] )) ||
_cipherroute__subcmd__provider_commands() {
    local commands; commands=(
'list:' \
'add:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cipherroute provider commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__provider__subcmd__add_commands] )) ||
_cipherroute__subcmd__provider__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute provider add commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__provider__subcmd__help_commands] )) ||
_cipherroute__subcmd__provider__subcmd__help_commands() {
    local commands; commands=(
'list:' \
'add:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cipherroute provider help commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__provider__subcmd__help__subcmd__add_commands] )) ||
_cipherroute__subcmd__provider__subcmd__help__subcmd__add_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute provider help add commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__provider__subcmd__help__subcmd__help_commands] )) ||
_cipherroute__subcmd__provider__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute provider help help commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__provider__subcmd__help__subcmd__list_commands] )) ||
_cipherroute__subcmd__provider__subcmd__help__subcmd__list_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute provider help list commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__provider__subcmd__list_commands] )) ||
_cipherroute__subcmd__provider__subcmd__list_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute provider list commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__route_commands] )) ||
_cipherroute__subcmd__route_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute route commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__tunnel_commands] )) ||
_cipherroute__subcmd__tunnel_commands() {
    local commands; commands=(
'start:' \
'stop:' \
'status:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cipherroute tunnel commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__tunnel__subcmd__help_commands] )) ||
_cipherroute__subcmd__tunnel__subcmd__help_commands() {
    local commands; commands=(
'start:' \
'stop:' \
'status:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cipherroute tunnel help commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__tunnel__subcmd__help__subcmd__help_commands] )) ||
_cipherroute__subcmd__tunnel__subcmd__help__subcmd__help_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute tunnel help help commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__tunnel__subcmd__help__subcmd__start_commands] )) ||
_cipherroute__subcmd__tunnel__subcmd__help__subcmd__start_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute tunnel help start commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__tunnel__subcmd__help__subcmd__status_commands] )) ||
_cipherroute__subcmd__tunnel__subcmd__help__subcmd__status_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute tunnel help status commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__tunnel__subcmd__help__subcmd__stop_commands] )) ||
_cipherroute__subcmd__tunnel__subcmd__help__subcmd__stop_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute tunnel help stop commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__tunnel__subcmd__start_commands] )) ||
_cipherroute__subcmd__tunnel__subcmd__start_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute tunnel start commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__tunnel__subcmd__status_commands] )) ||
_cipherroute__subcmd__tunnel__subcmd__status_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute tunnel status commands' commands "$@"
}
(( $+functions[_cipherroute__subcmd__tunnel__subcmd__stop_commands] )) ||
_cipherroute__subcmd__tunnel__subcmd__stop_commands() {
    local commands; commands=()
    _describe -t commands 'cipherroute tunnel stop commands' commands "$@"
}

if [ "$funcstack[1]" = "_cipherroute" ]; then
    _cipherroute "$@"
else
    compdef _cipherroute cipherroute
fi
