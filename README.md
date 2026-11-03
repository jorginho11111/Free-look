# Free Look — mod `.hmod` para o Horizon

[![Build Free Look .hmod](https://github.com/jorginho11111/Free-look/actions/workflows/build.yml/badge.svg)](https://github.com/jorginho11111/Free-look/actions/workflows/build.yml)

> ⚠️ **Leia isto primeiro.** A API oficial de mods sandboxed do Horizon
> (`horizon:mods@0.1.0`) **não tem** função para definir o yaw/pitch ou a
> perspectiva da câmera, **não permite** cancelar o F5 do jogo (as teclas de um
> mod nunca consomem o mapeamento vanilla) e **não entrega** delta de mouse /
> toque. Portanto **a câmera livre propriamente dita não pode ser feita em
> `.hmod`**. Em vez de entregar código que finge funcionar, este repositório
> entrega um `.hmod` que carrega, com **todas** as partes que a API permite, e
> diz isso claramente dentro do jogo, em inglês. O que falta e a alternativa
> exata estão em [`docs/API-LIMITATIONS.md`](docs/API-LIMITATIONS.md) e
> [`docs/NATIVE-API-PROPOSAL.md`](docs/NATIVE-API-PROPOSAL.md).

## O que o mod faz hoje

| Pedido | Situação |
| --- | --- |
| F5 vira free look, sem mudar para terceira pessoa | ❌ Impossível: `register-key` não consome o F5 e não há setter de câmera. O F5 do jogo continua trocando a perspectiva. |
| Câmera gira livremente sem alterar olhar/movimento do personagem | ❌ Impossível: `game.get-player()` é somente leitura; `render.set-zoom` mexe só no FOV; não há evento de delta de ponteiro. |
| Retorno suave à direção original | ✅ Lógica completa e testada (`Rig::step`, caminho angular mais curto, sem snap), pronta para um backend nativo. |
| Limite de ângulo vertical | ✅ `clamp_pitch` limita a ±80° (a câmera nunca vira de cabeça para baixo). |
| "Auto free look" e "Shift for free look" em inglês | ✅ Página em Options > Mods, botões `Auto free look: ON/OFF` e `Shift for free look: ON/OFF`, comandos `/freelook`. |
| Ambas desligadas → F5 intacto | ✅ Comportamento exato; também é o padrão após instalar. |
| Configurações persistem ao reiniciar | ✅ `storage` (capacidade oficial), com teste de "restart" no host simulado. |
| Todo o texto da interface em inglês | ✅ Todos os textos de UI, chat, HUD e feedback de comando estão em inglês. |
| Estrutura do mod de Zoom que já funciona | ✅ Mesmo `horizon-mod.toml` (`api = "0.1"`, `side = "client"`), mesmo SDK/WIT de `e7614d2`, mesmo `register-key`, mesma página `ui.set-settings`, mesmo empacotamento `horizon-mod.toml` + `mod.wasm`. |

Ao pressionar F5 com uma das opções ligadas, o mod mostra (uma vez) em inglês:
`Free Look: camera control is unavailable in this Horizon build. F5 keeps the game's perspective.`

## Instalar no Horizon Android

1. Baixe **`free_look.hmod`**:
   - na aba **Actions** → último run de *Build Free Look .hmod* → artefato
     `free-look-hmod`; ou
   - em **Releases**, se houver uma tag `v*`: o asset `free_look.hmod` tem uma
     URL HTTPS direta, que é o formato que o launcher aceita para importar.
   - Confira o SHA-256 com `SHA256SUMS.txt` do mesmo artefato.
2. No launcher do Horizon Android: **Mods → Import** (URL HTTPS do release, ou
   **Import file** se você copiou o arquivo para o celular).
3. **Installed → Free Look → Enable** (novos mods entram desligados).
4. Entre em um mundo: **Options → Mods → Free Look → Settings** para ligar
   `Auto free look` / `Shift for free look`. As teclas aparecem como *Key
   Binds* na mesma página e podem ser revinculadas.

**Requisitos:** APK 64-bit (`arm64-v8a` ou `x86_64`) com o runtime WASM de
`.hmod` — exatamente o mesmo runtime em que o seu mod de Zoom (`e7614d2`,
`horizon:mods@0.1.0`) funciona. O `main` atual do Horizon upstream removeu os
mods WASM e só aceita `.hnmod` nativos; nesse build este `.hmod` não carrega.
No Android oficial, `.hnmod` arbitrários não são compilados.

## Onde ficam as configurações

Tudo na pasta de jogo do perfil (a mesma que contém `options.txt`; no Android é
o armazenamento privado do app, por perfil):

| Arquivo | Conteúdo |
| --- | --- |
| `horizon-mods/.horizon-mod-data/free_look/auto_free_look` | `true` / `false` |
| `horizon-mods/.horizon-mod-data/free_look/shift_for_free_look` | `true` / `false` |
| `horizon-mods.txt` | `mod.free_look:true` (liga/desliga o mod), `key.free_look.free_look:key.keyboard.f5`, `key.free_look.shift_left:key.keyboard.left.shift`, `key.free_look.shift_right:key.keyboard.right.shift` (rebinds) |

As chaves de `storage` são gravadas por `storage.set` e relidas em `init()` —
portanto sobrevivem a reiniciar o jogo. Não há arquivo de configuração fora da
pasta do jogo.

## Comandos (inglês)

```
/freelook status
/freelook auto on|off
/freelook shift on|off
/freelook reset
```

## Regras de ativação implementadas

- Ambas OFF → o mod ignora o F5; o jogo faz o de sempre.
- `Auto free look` ON e `Shift for free look` OFF → F5 alterna free look.
- `Shift for free look` ON → **somente** Shift+F5; F5 sozinho fica com o jogo.
- As duas ON → a regra mais restritiva vence: só Shift+F5 (documentado na
  página de configurações).
- F5 durante o retorno suave cancela o retorno em vez de reativar.

> Observação honesta: como a API não permite consumir a tecla, mesmo no caso
> Shift+F5 o jogo **também** troca de perspectiva. Só um `.hnmod` com hook de
> teclado cancelável resolve isso (veja a proposta).

## Estrutura do projeto

```
horizon-mod.toml              manifesto (id free_look, api 0.1, client)
Cargo.toml                    workspace Rust
src/lib.rs                    mod: init, frame, comandos, settings, storage, input
src/camera.rs                 CameraOutput: ponto único (hoje no-op documentado)
crates/logic/                 máquina de estados pura + 9 testes unitários
vendor/horizon-mod-sdk/       SDK oficial verbatim de e7614d2 (igual ao Zoom)
wit/horizon.wit               WIT oficial completo (837 linhas)
wit/free-look.wit             mundo derivado, só as interfaces usadas
reference/e7614d2/            api_level.rs oficial (validação de funções)
tools/componentize/           wit_component::ComponentEncoder
tools/build.sh                build canônico + todos os checks
tools/package.py              gera free_look.hmod (ZIP) + SHA256SUMS.txt
tools/verify.mjs              imports/exports, nível de API, sem WASI, sem API inventada
tools/test-host.mjs           host simulado (jco): settings, persistência, F5/Shift
.github/workflows/build.yml   GitHub Actions → artefato .hmod pronto para baixar
docs/API-LIMITATIONS.md       o que falta na API, com fontes oficiais
docs/NATIVE-API-PROPOSAL.md   contrato do .hnmod + bridge para fazer funcionar
```

## Compilar localmente

```bash
rustup target add wasm32-unknown-unknown
npm ci
bash tools/build.sh
```

Saídas: `mod.wasm` (componente WASM), `free_look.hmod` (instalável) e
`SHA256SUMS.txt`. Os checks executados:

- `cargo test -p free-look-logic` — 9 testes puros;
- `node tools/verify.mjs mod.wasm` — todas as funções importadas existem no
  `api_level.rs` oficial, exports `events`/`ui-events`, sem WASI, sem `render`;
- `node tools/test-host.mjs` — ~25 checagens de comportamento no host simulado.

O **GitHub Actions** roda exatamente `tools/build.sh` em `ubuntu-latest` e
publica o `.hmod` como artefato (e como asset de release em tags `v*`).

## Licença

MIT — veja [`LICENSE`](LICENSE). O SDK vendorizado e o WIT são cópias do
repositório oficial do Horizon, usados apenas para fixar a compatibilidade com
a revisão `e7614d2`.
