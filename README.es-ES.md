# yqr

[![Benchmarks](https://img.shields.io/badge/benchmarks-live%20dashboard-blue?logo=rust&logoColor=white)](https://zoosky.github.io/yqr/dev/bench/)

`yqr` ("YAML query in Rust") es una herramienta de línea de comandos estilo jq para **YAML** que da **prioridad a la fidelidad**. Consulta *y* edita YAML preservando cada byte que no se le pida cambiar: comentarios, comillas, sangría, orden de claves y finales de línea sobreviven todos.

- **Lecturas byte-exactas, por defecto.** `yqr '.' file.yaml` reproduce la entrada exactamente — sin flag, sin reestructurar.
- **Edits quirúrgicos.** `yqr -i '.spec.replicas = 5' deploy.yaml` reescribe solo los bytes que el filtro apunta, o se niega: diffs limpios, garantizados. jq es solo JSON y no puede preservar el formato YAML en absoluto; yq edita en el lugar pero su documentación admite problemas de comentarios y espacios en blanco. yqr no cambia nada salvo el sitio de edición, o da error.
- **YAML nativo, sin round-trip a JSON.** El análisis y la emisión pasan por el motor [`noyalib`](https://crates.io/crates/noyalib) — el CST sin pérdidas detrás tanto de la ruta de lectura predeterminada como del pipeline `--normalize`; la CLI usa [`clap`](https://crates.io/crates/clap).

## Instalar / construir

Instala el crate publicado desde crates.io:

```sh
cargo install yqr
# binario en ~/.cargo/bin/yqr
```

O construye desde un checkout de código fuente (requiere la herramienta Rust **1.97.1**, fijada mediante `rust-toolchain.toml`):

```sh
cargo build --release
# binario en target/release/yqr
```

## Uso

```sh
yqr [OPCIONES] <FILTRO> [ARCHIVO]
yqr validate [--strict] [ARCHIVOS]...

Argumentos:
  <FILTRO>  El filtro estilo jq a aplicar (ej. '.foo.bar', '.items[]')
  [ARCHIVO]   Archivo YAML de entrada; lee stdin cuando se omite o '-'

Opciones:
  -r, --raw-output    Emite resultados de cadena sin comillas YAML
  -N, --normalize     Re-serializa la salida (elimina comentarios, canonicaliza escalares)
  -i, --in-place      Edita el archivo de entrada en el lugar (filtros mutadores solo)
  -h, --help          Imprimir ayuda
  -V, --version       Imprimir versión
```

### Ejemplos

```sh
# Acceso a campo
echo 'name: yqr
version: 1' | yqr .name
# => yqr

# Acceso anidado + indexación de arreglos
echo 'authors:
  - name: ada
  - name: linus' | yqr -r '.authors[0].name'
# => ada

# Indexación negativa (desde el final)
echo 'tags: [cli, yaml]' | yqr -r '.tags[-1]'
# => yaml

# Iterar una colección (un resultado por línea)
echo 'tags: [a, b, c]' | yqr -r '.tags[]'
# => a
#    b
#    c

# Composición por tubería
echo 'a: {b: {c: 42}}' | yqr '.a | .b | .c'
# => 42

# El `?` opcional suprime errores
echo 'name: yqr' | yqr '.name[]?'   # imprime nada, sale 0
```

## Lecturas que preservan bytes (predeterminado) y `--normalize`

**yqr preserva el formato por defecto.** Los nodos no modificados se emiten como sus **bytes de origen originales**, por lo que el filtro de identidad reproduce la entrada exactamente: comentarios, comillas, sangría y finales de línea sobreviven. Pasa **`--normalize`** (`-N`) para re-serializar la salida en su lugar, lo que canoniza escalares y elimina comentarios.

Las lecturas que preservan bytes son impulsadas por el CST sin pérdidas de [`noyalib`](https://crates.io/crates/noyalib): el único y único motor YAML de yqr.

```bash
# La identidad reproduce el archivo byte a byte: comentarios, líneas en blanco,
# comillas, escalares bloque, CRLF, BOM y flujos multi-documento sobreviven
yqr '.' config.yaml | diff config.yaml -   # no hay diff (no se necesita flag)

# Las proyecciones conservan la escritura original
echo "zip: 007" | yqr '.zip'      # => 007   (no 7)
echo "s: 'hi'"  | yqr '.s'        # => 'hi'  (comillas conservadas)

# --normalize re-serializa (con pérdida: elimina comentarios, canoniza escalares)
echo "zip: 007" | yqr --normalize '.zip'   # => 7    (re-tipado)
yqr --normalize '.' config.yaml            # comentarios eliminados, escalares canonizados
```

Los resultados que se calculan en lugar de seleccionarse (y nodos que un motor no puede dirigir fielmente: entradas fusionadas mediante `<<`, referencias de alias) retroceden al renderizado tipado normal. Las entradas multi-documento ejecutan el filtro contra cada documento. `-r` mantiene su significado habitual e imprime valores de *cadena*.

Notas de fidelidad:

- Las **colecciones bloque anidadas** proyectadas se emiten a su sangría original (la rebanada se extiende hasta el inicio de la línea), por lo que la salida está uniformemente sangrada y se vuelve a analizar al valor seleccionado.
- **Entrada vacía** no produce salida en el modo predeterminado (preservación de bytes) (identidad byte con el archivo vacío), donde `--normalize` imprime `null`.
- El modelo de valor del backend noyalib tiene **claves de mapeo solo de cadena**: las claves no cadena (`true:`, `8080:`) se emparejan por escritura; las claves distintas que colisionan tras la conversión a cadena (`1` y `"1"`) se rechazan con un error. Las claves duplicadas resuelven con el último ganador y emiten los bytes reales de la última ocurrencia. Los escalares de bloque con chomping de mantenimiento (`|+`) conservan sus líneas en blanco finales conservadas, las referencias de alias proyectan los bytes reales del ancla, las colecciones bloque comienzan en la sangría de su primera línea, y los finales de línea solo CR de clásico Mac se aceptan.

## Ediciones quirúrgicas (`=`, `+=`, `del`, `-i`)

yqr también puede **editar** YAML, no solo leerlo: y cambia solo los bytes que el filtro apunta, dejando cada otro byte (comentarios, sangría, comillas, orden de claves) intacto, o se niega. Los edits siempre pasan por el motor de fidelidad, por lo que un filtro mutador es byte-exacto excepto en el sitio de edición.

La superficie de mutación:

| Filtro                    | Significado                                              |
|---------------------------|----------------------------------------------------------|
| `<ruta> = <valor>`        | Reemplazar el escalar en `ruta` (comillas emparejadas al estilo) |
| `<ruta>.<nuevaclave> = <valor>` | Agregar una nueva entrada de mapeo bajo un mapeo existente   |
| `<ruta> += <valor>`       | Agregar un artículo a la secuencia bloque en `ruta`        |
| `del(<ruta>)`             | Eliminar la entrada bloque en `ruta` (de una o varias líneas) |

`<valor>` es un literal escalar (`5`, `1.5`, `"web"`, `true`, `false`, `null`) o una ruta con raíz `.` que copia el valor encontrado en otra ubicación.

```bash
# Reemplazar un valor: el comentario y cada otra línea se conservan literalmente
echo 'spec:
  replicas: 3   # conserva
  image: web' | yqr '.spec.replicas = 5'
# => spec:
#      replicas: 5   # conserva
#      image: web

# Agregar a una secuencia bloque en la sangría correcta
yqr '.spec.ports += 9090' deploy.yaml

# Agregar una nueva clave, eliminar una entrada (un bloque anidado/multilínea se cierra limpiamente)
yqr '.metadata.env = "prod"' deploy.yaml
yqr 'del(.metadata.labels)' deploy.yaml
yqr 'del(.spec.template)' deploy.yaml

# Editar el archivo en el lugar (reescrito atómicamente: archivo temporal + renombrar)
yqr -i '.spec.replicas = 5' deploy.yaml
git diff deploy.yaml   # toca solo esa una línea
```

Garantías y límites:

- **Integridad estructural.** Una edición cuyo resultado no se volvería a analizar a una estructura diferente es **rechazada** (salida 5) en lugar de emitida; bajo `-i` el archivo se deja sin cambios.
- **No coincidencia es un no-op.** Un filtro que no coincide con ningún nodo tiene éxito y deja el documento sin cambios (semántica jq/yq), por lo que `del(.x)` sobre un lote de archivos no falla los que no tienen `.x`.
- **`-i` necesita un archivo.** Usar `--in-place` con stdin, o con un filtro de solo lectura, es un error (diagnosticado antes de leer cualquier entrada). Las escrituras son atómicas (archivo temporal + `fsync` + renombrar) y editan *a través* de un enlace simbólico hacia el archivo real; el modo original se conserva. Dueño/grupo, contexto SELinux, ACL, atributos extendidos y enlaces duros **no** se transfieren tras el reemplazo: el mismo intercambio archivo-temporal+renombrar que hace `sed -i`.
- **Multi-documento.** La edición se aplica a cada documento cuya ruta se resuelve; los demás se emiten byte-idénticamente.
- **RHS escalar solo.** Los valores de `=`, `+=` y nuevas claves son escalares (número, cadena, booleano, nulo) o una ruta que copia un escalar; un RHS de colección es rechazado.
- **Eliminar estructural.** `del` elimina entradas bloque multi-línea y anidadas también, no solo de una sola línea: cierra las líneas de la entrada y deja cada byte sobreviviente idéntico. Eliminar la *única* entrada de un bloque (que lo vaciaría) o un artículo de una *colección de flujo* (`[a, b]`) es rechazado con un mensaje claro.
- **Operaciones no admitidas.** Las actualizaciones calculadas (`|=`), el renombrado de claves, y el reorden de secuencias / edición de comentarios fallan cada una con un mensaje claro.

## Validar archivos (`yqr validate`)

<!-- Feature f012 -->

Después de una edición: quirúrgica, hecha a mano o hecha por un agente, un solo comando responde si un archivo sigue siendo YAML correcto:

```sh
yqr validate deploy.yaml config.yaml   # silencioso, sale 0 cuando cada archivo es válido
yqr validate --strict deploy.yaml      # también marcar duplicados de claves de mapeo
yqr validate - < input.yaml            # stdin es explícito: '-', a lo sumo una vez
```

Una lista de archivos vacía es un error de uso (salida 2), nunca un retroceso silencioso a stdin: un gate de CI cuyo glob salió vacío debe fallar en voz alta, no reportar "todo válido" sobre nada.

Los fallos son diagnósticos estilo compilador en stderr, con un código estable, una ubicación `file:line:column` clicable siempre que se conozca una posición (la mayoría de fallos; algunos errores de analizador no llevan ninguna), la línea de origen ofensor, y una solución sugerida cuando existe:

```text
error[Y001]: expected a node but found StreamEnd
  --> deploy.yaml:3:1
  |
3 | b: [1,
  | ^
```

Una pasada certifica más que "se analiza": los documentos analizados deben reproducir la entrada byte a byte: el mismo invariante de integridad detrás de las lecturas de fidelidad de yqr.

| Código | Hallazgo                                                        | Modo       |
|--------|-----------------------------------------------------------------|------------|
| `Y001` | La entrada no es YAML bien formado                              | predeterminado |
| `Y002` | Los documentos analizados no reproducen la entrada byte a byte  | predeterminado |
| `Y003` | Los bytes de entrada no son UTF-8 válido                        | predeterminado |
| `Y101` | Clave de mapeo duplicada (silenciosamente último-gana en lecturas ordinarias) | `--strict` |
| `Y102` | Claves distintas colisionan tras la conversión a cadena (`1:` vs `"1":`) | predeterminado |

`--strict` encuentra **todos** los duplicados en una sola pasada: mapeo anidados, mapeo de flujo, reescrituras entrecomilladas de la misma clave, y claves `<<` de fusión duplicadas incluidas: cada una con las posiciones de ambas ocurrencias.

Los códigos de salida son scripteables: `0` cuando cada entrada es válida, `1` cuando alguna entrada tiene hallazgos de validación, `5` cuando una entrada no puede leerse: el código más alto aplicable gana, y cada entrada se comprueba en una sola pasada (errores de uso como no entradas o un `-` repetido salen 2). Un archivo con marcadores de conflicto de fusión sin resolver (`<<<<<<<`) obtiene una pista dedicada anclada en el primer marcador.

## Filtros de consulta

| Filtro    | Significado                                          |
|-----------|------------------------------------------------------|
| `.`       | Identidad                                            |
| `.foo`    | Acceso a campo (`.["foo"]` para claves no palabras simples) |
| `.a.b`    | Acceso a campos anidados                             |
| `.[n]`    | Índice de arreglo (`.[-1]` cuenta desde el final)    |
| `.[]`     | Iterar elementos de secuencia / valores de mapeo     |
| `a \| b`  | Tubería                                              |
| `f?`      | Suprimir errores de `f`                              |

Planeado: construcción de objetos/arreglos, funciones incorporadas (`length`, `keys`, `select`, `map`, …), aritmética, modo multi-documento/slurp, y más. Ver la especificación.

## Sitio web

El sitio web del proyecto: rutas de instalación, recetas para ejecutar yqr contra salida de `kubectl` y otro YAML (configuraciones de CI, archivos Compose, playbooks de Ansible, especificaciones OpenAPI, reglas de alerta, configuración de aplicación), además del árbol de especificación completo navegable, está en 
**[zoosky.github.io/yqr](https://zoosky.github.io/yqr/)**.

Es un sitio de Accent CMS: un CMS de markdown de un solo binario ([fuente](https://github.com/AccentCMS/accent)), construido desde [`docs/`](docs/) con [`specs/`](specs/) montado en `/specs`, desplegado por el flujo de trabajo `Website` en cada push a `main`. Vista previa local: `cd docs && accent serve`.

## Arquitectura

```
filtro ──▶ lexer ──▶ parser ──▶ Ast ──▶ evaluador ──▶ Value(s) ──▶ YAML
YAML   ──▶ noyalib::from_str ──▶ Value ──┘
```

| Módulo          | Responsabilidad                                    |
|-----------------|----------------------------------------------------|
| `src/lexer.rs`  | Cadena de filtro → tokens                          |
| `src/parser.rs` | Tokens → `Ast`                                     |
| `src/ast.rs`    | Definiciones de nodos AST de filtro                |
| `src/eval.rs`   | `Ast` × `Value` → flujo de `Value`                 |
| `src/value.rs`  | Modelo `Value` de yqr (convierte a/desde `noyalib`) |
| `src/fidelity/` | Motor de lectura que preserva bytes (lecturas predeterminadas) + capa de escritura (`src/fidelity/write.rs`) |
| `src/error.rs`  | `YqrError` + mapeo de código de salida estilo jq   |
| `src/cli.rs`    | Análisis de argumentos con `clap`                  |
| `src/lib.rs`    | API pública (`eval_str`, `render`)                  |
| `src/main.rs`   | Entrada binaria + mapeo de código de salida        |

## Pruebas

```sh
cargo test            # pruebas unitarias + integración + CLI
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

- Las **pruebas unitarias** viven junto a cada módulo.
- **`tests/integration.rs`** ejercita la API de biblioteca pública de extremo a extremo.
- **`tests/cli.rs`** ejecuta el binario compilado contra entrada canalizada.

## Referencias

Las referencias de Criterion viven en `benches/` (`cargo bench --bench eval`). Cada push a `main` las ejecuta en CI y publica los resultados en un historial rastreado:

**[Panel de referencias en vivo](https://zoosky.github.io/yqr/dev/bench/)** — rendimiento en el tiempo, con alertas en regresiones del 30%.

## Licencia

Licenciado bajo cualquiera de

- Apache License, Versión 2.0 ([LICENSE-APACHE](LICENSE-APACHE) o
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) o
  <http://opensource.org/licenses/MIT>)

a tu elección.

## Contribución

A menos que declares explícitamente lo contrario, cualquier contribución intencionalmente presentada para inclusión en la obra por ti, como se define en la licencia Apache-2.0, se licenciará doblemente como arriba, sin términos o condiciones adicionales.
