# 📖 Glosaric CLI (`glosaric`)

> **Navegación de código determinista y generador de glosarios duales para desarrolladores e IAs.**  
> Erradica el escaneo ciego (`grep` masivo), previene el *Context Rot* en agentes cognitivos y reduce hasta un 75% el consumo de tokens en tareas de ingeniería.

---

## ⚡ ¿Qué problema resuelve?

Cuando un desarrollador o una IA (Cursor, Copilot, Claude Code, Antigravity) interactúa con un repositorio mediano o grande, suele recurrir a búsquedas ciegas (`grep`, `find`) o a leer decenas de archivos completos. Esto provoca:
* **Desperdicio masivo de tokens y lentitud.**
* **Context Rot:** pérdida de foco por saturación de contexto con código irrelevante.
* **Deuda técnica invisible:** funciones y structs huérfanos que nadie documenta.

**Glosaric** escanea el código en **< 10 milisegundos**, extrae las APIs públicas (`pub fn`, `struct`, `enum`, `type`, `const`) y genera una **hoja única de navegación** en formato Markdown estándar con estructura dual:

```markdown
- `funcion(params)` | Explicación técnica precisa / tipos | Traducción humana / para qué sirve en la práctica
```

---

## 🚀 Comandos Principales

### 1. Inicializar un Glosario (`glosaric init`)
Escanea el código fuente recursivamente y genera `docs/GLOSARIC.md`:
```bash
glosaric init .
```
O sincroniza directamente con tu Bóveda Obsidian:
```bash
glosaric init . --vault
```

### 2. Auditar Deuda Técnica en CI/CD (`glosaric check`)
Compara el código fuente contra el Glosario. Si alguien creó funciones públicas sin documentarlas, falla con código de salida `1`:
```bash
glosaric check .
```
Ideal para GitHub Actions y pre-commit hooks:
```yaml
- name: Audit Glosaric Sync
  run: glosaric check . --strict
```

### 3. Sincronización Inteligente No Destructiva (`glosaric sync`)
Detecta nuevos símbolos en el código y los anexa automáticamente **sin sobreescribir ni borrar tus traducciones humanas existentes**:
```bash
glosaric sync . --backup
```

### 4. Búsqueda Relámpago en Terminal (`glosaric query`)
Localiza cualquier símbolo, tipo o concepto en < 3 milisegundos:
```bash
glosaric query "sync" .
```
O en formato JSON estructurado para extensiones o agentes de IA:
```bash
glosaric query "sync" . --json
```

---

## 🛡️ Principios de Diseño
1. **Cero pérdida de datos:** Las descripciones escritas a mano por desarrolladores están blindadas.
2. **Cero dependencias pesadas:** Binario único nativo en Rust de ~3 MB.
3. **Multi-idioma e i18n:** Soporte UTF-8 completo para comentarios y descripciones en cualquier idioma.
4. **Simetría Humano-IA:** Formato Markdown optimizado tanto para lectura humana rápida (`Ctrl + F` < 15s) como para inyección contextual en prompts de LLMs.

---

## 📄 Licencia
MIT / Apache 2.0
