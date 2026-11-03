# Alternativa: como o Free Look real pode existir no Horizon Android

> Tudo neste arquivo é **proposta de API nova**. Nenhuma das funções/hooks
> abaixo existe hoje; elas são o contrato que um `.hnmod` nativo teria de
> implementar para que o `.hmod` deste repositório passe a girar a câmera.
> O `.hmod` entregue **não** chama nada disso.

## Caminho recomendado: `.hnmod` + mod bridge + APK de um fork

Modelo oficial do Horizon Android (guia de mods 0.7.8 dev 1009):

```
.hmod (sandboxed, importado por URL, sem recompilar)
   -- bridge::call("freelook", "set_view", JSON) -->
game  -->  .hnmod nativo compilado dentro do APK  -->  engine (câmera, input)
```

1. Escrever um `.hnmod` (Rust + horizon-mixins) fixado ao commit do Horizon
   usado pelo APK. Ele ganha três hooks novos:
   - `native_key` com `consumed: &mut bool` para **F5** (ou um
     `claim-key` no input do jogo);
   - `native_pointer(dx, dy)` com o delta do mouse / swipe de câmera do
     Android;
   - `native_camera_angles(partial, view: &mut Option<CameraView>)`, chamado
     uma vez por frame antes de montar a matriz de visão, com
     `CameraView { yaw: f32, pitch: f32, distance: f32 }`. Deve alterar somente
     a câmera: `player.yaw`, `player.pitch`, o vetor de movimento e o estado
     `key.togglePerspective` permanecem intactos.
2. Publicar nesse `.hnmod` uma API de bridge chamada `freelook`, versão 1:

   | Tipo | Nome | Argumento / retorno |
   | --- | --- | --- |
   | `call` | `enable` | `{"yaw":0.0,"pitch":0.0}` → `""` |
   | `call` | `set_view` | `{"yaw":...,"pitch":...}` → `""` (uma vez por frame, durante a interpolação de retorno) |
   | `call` | `disable` | `""` → `""` (devolve a câmera ao jogo) |
   | `call` | `claim_f5` | `{"on":true}` → `""` |
   | `event` | `pointer` | `{"dx":12.5,"dy":-3.0}` (assinado na `init`) |
   | `ask` | `f5` | `{"shift":true}` → `None` deixa o F5 vanilla; `"cancel"` impede a troca de perspectiva |

3. No `.hmod`, trocar **apenas** `src/camera.rs`:

```rust
// Proposta: NÃO compila hoje (bridge não existe em e7614d2 / api_level 7).
pub struct BridgeCamera;
impl CameraOutput for BridgeCamera {
    fn set_view(&self, yaw: f32, pitch: f32) -> bool {
        let args = serde_json::json!({"yaw": yaw, "pitch": pitch}).to_string();
        bridge::call("freelook", "set_view", &args).is_ok()
    }
    fn clear(&self) -> bool {
        bridge::call("freelook", "disable", "").is_ok()
    }
}
```

   e ligar os eventos de ponteiro a `Rig::look(dx, dy, sensitivity)`. Toda a
   máquina de estados (limite vertical de ±80°, retorno suave, regras de F5 e
   Shift, persistência) **já está pronta e testada** em `crates/logic`.

4. Manifesto da variante com bridge (usa o SDK do fork Android, não o de
   `e7614d2`):

```toml
id = "free_look"
api = "0.1"
side = "client"
capabilities = ["game-read", "chat", "hud", "storage", "ui", "input", "bridge"]
experimental = ["ui", "input", "bridge"]
```

   `bridge::version("freelook")` deve ser consultada na `init()`; sem o `.hnmod`,
   o mod mostra a mesma mensagem em inglês de hoje, em vez de falhar.

5. Compilar no Android: o APK oficial do `Vasysik/horizon-android` **não**
   compila `.hnmod` arbitrários ("Source-only .hnmod mods require a separately
   compiled client"). Use um fork com a opção **Build APK on your fork**
   (`JAK-cool162/horizon-android` 0.7.8 dev 1009+, que já traz o mod bridge
   nível 9 e o `extras_api`), adicione o `.hnmod` ao build e instale esse APK
   (arm64-v8a ou x86_64, 64-bit). Depois importe o `.hmod` normalmente.

## Caminho upstream (desktop / contribuição ao Horizon)

Abrir dois PRs no `horizon-rust/horizon` (os hooks são versionados por
`tools/native-hook-fragments/`):

- `ViewportEvent.ComputeCameraAngles` — está **"Not started"** no
  [issue #4953](https://github.com/horizon-rust/horizon/issues/4953);
- um `InputEvent.Key` **cancelável** (hoje existe, mas não há contrato de
  consumo para `key.togglePerspective`) e um evento de delta de ponteiro.

`native_camera_shake` ([PR #5412](https://github.com/horizon-rust/horizon/pull/5412))
não serve: é limitado a 10°, sem direção e decai. `native_camera_distance`
([PR #5992](https://github.com/horizon-rust/horizon/pull/5992)) só muda a
distância da câmera em terceira pessoa.

## Alternativa provisória sem reconstruir o APK (não é free look)

Se a ideia for apenas "olhar ao redor" enquanto o personagem continua parado,
o máximo que um `.hmod` consegue hoje é um **"Free Look Lite"**:

- deixar o F5 vanilla alternar para terceira pessoa;
- usar `render.set-zoom` para aproximar/afastar o FOV;
- usar `hud`/`ui` para um indicador e um botão de "reset".

Isso **não** desacopla a câmera da direção do jogador e, por isso, não foi
embutido neste mod para não fingir que o comportamento pedido funciona.

## O que já está pronto aqui

| Item | Onde |
| --- | --- |
| Manifesto, SDK e WIT idênticos ao Zoom que carrega | `horizon-mod.toml`, `vendor/horizon-mod-sdk/`, `wit/` |
| Registro de teclas, página de settings em inglês, persistência | `src/lib.rs` |
| Regras de F5 / Shift+F5 / ambas OFF | `crates/logic/src/lib.rs` (`f5_action`) |
| Limite vertical ±80° e retorno suave | `crates/logic/src/lib.rs` (`clamp_pitch`, `Rig::step`) |
| Ponto único onde a câmera nativa entra | `src/camera.rs` (`trait CameraOutput`) |
| CI que gera o `.hmod` | `.github/workflows/build.yml` |
