# Por que o Free Look (câmera livre) não é possível em `.hmod`

> Resumo em inglês / English summary: Horizon's sandboxed `.hmod` API
> (`horizon:mods@0.1.0`) has **no function to set the camera's yaw, pitch or
> perspective**, **no cancellable/claimable key input** (a mod's keys never
> consume the game's own mapping), and **no pointer-motion event**. `F5`
> therefore cannot be intercepted and the camera cannot be detached from the
> player's facing. The shipped `.hmod` implements every part the API allows and
> says so in game; `NATIVE-API-PROPOSAL.md` specifies the native API needed.

## 1. Fontes oficiais lidas antes de escrever o código

| Fonte | O que foi consultado |
| --- | --- |
| [`horizon-rust/horizon` → `docs/mods.md`](https://github.com/horizon-rust/horizon/blob/main/docs/mods.md) e [Mods and data packs](https://playhorizon.dev/docs/mods) | formato `.hmod`, manifesto, capacidades, separação entre content mod e native mod |
| WIT oficial `horizon:mods@0.1.0`, cópia literal em [`vendor/horizon-mod-sdk/wit/horizon.wit`](../vendor/horizon-mod-sdk/wit/horizon.wit) (revisão `e7614d2`, a mesma do Zoom que funciona) | **todas** as funções disponíveis para um mod WASM |
| [`reference/e7614d2/api_level.rs`](../reference/e7614d2/api_level.rs) | lista fechada das 46 funções e seus níveis de API (LEVEL = 7) |
| [Guia oficial de mods do Horizon Android 0.7.8 dev 1009](https://github.com/JAK-cool162/horizon-android/releases/tag/v0.7.8-dev.1009) (`horizon-extras-api-guide.txt`, Apêndice A: referência gerada completa) | confirma que a geração mais nova do Android tem **as mesmas** interfaces, mais `bridge` |
| [Horizon Android `docs/android.md`](https://github.com/Vasysik/horizon-android/blob/android/docs/android.md) | importação de `.hmod` por URL HTTPS, 64-bit, `.hnmod` não suportado no APK oficial |
| [PR #2013 — "Mod API: zoom the camera and take the mouse wheel"](https://github.com/ludovic111/horizon/pull/2013) | regra explícita: *"a mod's keys never consume: vanilla's mapping on the same key still runs"*; só `claim-wheel` e `claim-items` consomem input |
| [Hook points and helpers](https://playhorizon.dev/docs/mod-code) e [issue #4953](https://github.com/horizon-rust/horizon/issues/4953) | estado dos hook points nativos (`.hnmod`) no upstream |
| [`key_names.rs`](https://github.com/horizon-rust/horizon/blob/main/crates/horizon-client-input/src/key_names.rs) e [`keybinds.rs`](https://github.com/horizon-rust/horizon/blob/main/crates/horizon-client-input/src/keybinds.rs) | nomes reais `key.keyboard.f5`, `key.keyboard.left.shift`, `key.keyboard.right.shift` e o mapeamento vanilla `key.togglePerspective` |

## 2. O que a API realmente oferece (e nada além disso)

Interfaces de cliente em `horizon:mods@0.1.0` (todas as funções, conforme o WIT):

| Interface | Funções | Serve para o Free Look? |
| --- | --- | --- |
| `game` | `get-player()` (**leitura** de x/y/z/yaw/pitch/health/food/dimension), `get-clock()`, `raycast()` | Só para **ler** a direção do jogador. Não há `set-player`, `set-yaw` ou `set-pitch`. |
| `render` | `set-hud`, `gui-size`, `shake`, `set-corner`, `clear-corner`, `text-width`, `frame-stats`, `set-zoom`, `set-minimap`, `set-far-terrain` | `set-zoom` muda **apenas o FOV**; `shake` é um solavanco aleatório que decai e é limitado; as demais são overlays 2D. Nenhuma gira a câmera nem muda a perspectiva. |
| `input` | `claim-items`, `register-key`, `held`, `poll`, `claim-wheel`, `mapping`, `clicks-per-second` | `register-key` **ouve** teclas, mas o mapeamento vanilla continua rodando. `poll` só devolve `key-down`, `key-up`, `wheel`, `use-start`, `use-stop` — **não há delta de mouse/toque**. `mapping` só aceita `key.forward`, `key.left`, `key.back`, `key.right`, `key.jump`, `key.sneak`, `key.sprint`, `key.attack`, `key.use` (não `key.togglePerspective`). |
| `ui` / `ui-events` | `open-screen`, `close-screen`, `set-settings`, `select`, `closed`, `settings-opened` | Telas e página de configurações. Abrir uma tela não cancela o F5 que já foi processado. |
| `storage` | `get`, `set`, `delete` | Persistência das configurações. |
| `chat`, `hud`, `command`, `log`, `recipes`, `browser`, `players`, `world`, `bridge` | — | Nenhuma controla câmera ou F5. `bridge` (API nível 9, só nos dev builds do Android) apenas chama funções que um `.hnmod` nativo **já tenha registrado**; o `extras` v3 oferece opções de vídeo e funções de servidor, não câmera. |

## 3. O que falta, exatamente

1. **Um setter de orientação/perspectiva da câmera.** Algo como
   `render.set-camera(view { yaw, pitch, distance, perspective })` (ou um hook
   nativo `native_camera_angles`) que altere **a matriz de visão** sem tocar em
   `player.yaw` / `player.pitch`, no vetor de movimento nem no ciclo F5. Hoje só
   existe leitura (`game.get-player`) e FOV (`render.set-zoom`).
2. **Input de tecla cancelável / "claim-key".** `register-key` documenta que
   *"the game's own mapping on that key still runs"*. É preciso um
   `input.claim-key(name, on)` (ou `consumed: &mut bool` no hook de teclado)
   para impedir que o F5 do jogo troque para terceira pessoa. Só `claim-wheel`
   e `claim-items` consomem input hoje.
3. **Eventos de movimento relativo do ponteiro.** Para girar uma câmera livre
   são necessários `dx`/`dy` do mouse (e, no Android, do swipe de câmera ou do
   controle de toque). `input.poll()` não entrega isso; `mapping()` só diz se
   uma tecla de movimento está pressionada.
4. **Uma API nativa que ligue os dois lados.** No Android, um `.hmod` só chama
   o que um `.hnmod` compilado dentro do APK publicar via `bridge`. O APK
   oficial do Horizon Android **não compila `.hnmod` arbitrários**; é preciso
   um fork com "Build APK" (o fork `JAK-cool162/horizon-android` oferece isso).

Sem os itens 1–3 não existe "free look"; sem o item 4 não há como implementá-los
no Android sem reconstruir o APK.

## 4. Por que não dá para improvisar com o que existe

- `render.set-zoom({factor, smooth, turn})`: divide o FOV e escala a
  sensibilidade de giro. Não desacopla a câmera do corpo; é exatamente o que o
  mod de Zoom usa.
- `render.shake(strength, seconds)`: gira a *view* momentaneamente, mas é um
  jolt sem direção, que decai, escalado por "Distortion Effects". No upstream é
  limitado a 10° ([PR #5412](https://github.com/horizon-rust/horizon/pull/5412)).
- `command.run("/tp ... @s ...")` ou equivalentes: mudam **o jogador**, o que
  viola "sem mudar a direção em que o personagem olha ou anda".
- `ui.open-screen`: abre um menu; o F5 já terá sido processado e, além disso,
  telas abertas liberam as teclas do mod.
- `input.register-key("...", "key.keyboard.f5")`: recebemos o evento, mas o
  jogo **também** recebe e troca a perspectiva. É por isso que `F5` sozinho
  continua com o comportamento normal quando "Shift for free look" está ON.

## 5. Estado no upstream (alternativa nativa)

Os hook points nativos do Horizon evoluíram, mas **ainda não** cobrem isso:

- Existem `native_key`, `native_zoom`, `native_camera_shake`,
  `native_camera_distance` ([PR #5992](https://github.com/horizon-rust/horizon/pull/5992))
  e `InputEvent.Key` ([issue #4953](https://github.com/horizon-rust/horizon/issues/4953)).
- `ViewportEvent.ComputeCameraAngles` (o hook que permitiria ângulos de câmera
  independentes) está listado como **"Not started"** no mesmo issue #4953.
- `native_camera` (leitura das matrizes) é apenas uma solicitação em aberto
  ([issue #5276](https://github.com/horizon-rust/horizon/issues/5276)).

Ou seja: mesmo um `.hnmod` no desktop precisaria de um hook novo. A
especificação completa do que pedir/implementar está em
[`NATIVE-API-PROPOSAL.md`](NATIVE-API-PROPOSAL.md).

## 6. Existe alguma versão em que a câmera é suportada?

**Em `.hmod` (sandboxed): não, em nenhuma versão.** A lista fechada de funções
de `horizon:mods@0.1.0` ([`reference/e7614d2/api_level.rs`](../reference/e7614d2/api_level.rs),
níveis 0–7) nunca teve um setter de câmera, e a referência gerada mais recente
do Android (dev 1013, `docs/api/mods-reference.md` do fork JAK) continua com
apenas `render.shake` e `render.set-zoom` relacionados à câmera. A API de mods
só **ganha** funções; não existe uma revisão antiga com um `set-camera`
escondido. O fork WASM mais ambicioso — `Stink-O/horizon`,
[`orbital-mod-api-10` (03/11/2026)](https://github.com/Stink-O/horizon/releases/tag/orbital-mod-api-10) —
acrescenta um *post pass* WGSL que **lê** a câmera e `world.carry-player`
(que gira **o jogador**, até ±6°/tick), mas também não define ângulos de
câmera, não entrega delta de mouse e não consome teclas.

**Em `.hnmod` (native): sim.** Um mod nativo é Rust injetado no código do jogo
por `horizon-mixins`, então ele pode patchar diretamente
`Game::apply_mouse_look` e `perspective::Game::camera_view`. Já existe um mod
assim no registro oficial:

| Mod | Entrega | Observações |
| --- | --- | --- |
| [Freelook 0.3.1](https://playhorizon.dev/mods/freelook) — Krzysix17, 31/10/2026, MIT, pin `a491181` | [`.hnmod` (5,7 KB)](https://github.com/horizon-rust/horizon-mods/releases/download/freelook-0.3.1/freelook-0.3.1.hnmod), [código-fonte](https://github.com/Krzysix17/horizon-freelook) | Segura **X** (revinculável em Options > Mods) para uma câmera em terceira pessoa que gira com o mouse sem mudar o olhar/movimento do personagem; a roda muda a distância; soltar volta. |
| `freecam`, `freecam_plus`, `fair_freecam` (registro oficial) | `.hnmod` | Câmera que **sai do corpo** e voa com WASD; são freecam, não free look em órbita do personagem. |

Diferenças em relação ao que foi pedido aqui: o Freelook é *hold* (não é F5
toggle), não tem as opções "Auto free look"/"Shift for free look", não
persiste configurações além do rebind de tecla, e o retorno é instantâneo
(não suave). O clamp vertical é o do vanilla (±90°) em vez de ±80°. O código é
MIT, então dá para adaptar — veja [`NATIVE-API-PROPOSAL.md`](NATIVE-API-PROPOSAL.md).

**No Android:** o APK oficial (`Vasysik/horizon-android`) **não compila
`.hnmod`**. O fork `JAK-cool162/horizon-android` (dev 1009+, hoje 1013) tem
*Import .hnmod* e **Build APK on your fork** (ou o workflow "Android dev APKs"
com o input `native_mods`), com `--allow-version-mismatch` para mods pinados em
outro commit; leva ~40–70 min e gera um APK dev. Verificamos que as duas
funções que o Freelook hooka (`Game::apply_mouse_look` e
`perspective::Game::camera_view`) **ainda existem com a mesma assinatura** no
fonte desse fork, e o módulo é injetado em `crates/horizon-client/src/game.rs`
(biblioteca compilada no Android, não `main.rs`), então não cai na regra
"Desktop only". Ainda assim **não é garantido**: o pin `a491181` é de outro
commit e o `horizon-mixins` pode recusar algum alvo.

**No upstream atual (desktop):** `native-hooks.json` tem `camera_shake`
(rotação aditiva, limitada a ~10°) e `camera_distance` (distância em terceira
pessoa), ambos `listen`; há um fragmento pendente `camera_read` para **ler**
as matrizes. `ViewportEvent.ComputeCameraAngles` segue "Not started" no
[issue #4953](https://github.com/horizon-rust/horizon/issues/4953). Ou seja:
nem os hook points nativos dão os ângulos — o Freelook funciona porque usa
injeções `HEAD` canceláveis em funções internas, não uma API pública.

## 7. O que este repositório entrega, então

Um `.hmod` **que carrega** (mesma estrutura, SDK, WIT, manifesto, registro de
teclas e tela de configurações do Zoom que já funciona) e que implementa
tudo o que é possível:

- configurações **"Auto free look"** e **"Shift for free look"** em inglês,
  persistidas em `storage` e restauradas ao reiniciar;
- registro de `F5`, `Shift` esquerdo e `Shift` direito (re vinculáveis em
  Options > Mods);
- a regra exata: ambas OFF → F5 intacto; Shift ON → só Shift+F5; F5 alterna
  liga/desliga;
- limite vertical de ±80° e retorno suave com caminho angular mais curto, como
  máquina de estados pura e testada (`crates/logic`);
- página de configurações, `/freelook`, HUD e chat **em inglês** explicando
  claramente que o controle de câmera não está disponível neste build.

`src/camera.rs` é intencionalmente um *no-op*: ele **não chama nenhuma função
inventada**. `tools/verify.mjs` falha o build se o componente importar qualquer
função que não exista no `api_level.rs` oficial, ou qualquer função de `render`.
