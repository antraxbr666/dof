---
name: dof-release
description: >
  Use SEMPRE que houver qualquer modificação nos arquivos src/, install.sh, README.md ou .github/workflows/ deste projeto.
  Esta skill controla o ciclo de versionamento e release do projeto dof.
  Toda alteração DEVE incrementar a versão no Cargo.toml,
  criar commit com mensagem descritiva, tag e push para disparar a GitHub Release automaticamente.
---

# dof-release — Versionamento e Release

## Regra Inviolável

**Toda modificação no projeto DEVE resultar em uma nova versão publicada.**

A fonte da verdade da versão é `Cargo.toml` → campo `version`.

---

## Fluxo Obrigatório

Quando o usuário pedir para alterar qualquer coisa no projeto:

```
1. Fazer a modificação
2. Incrementar versão no Cargo.toml
3. Build e verificação: cargo build --release
4. Commit com mensagem descritiva (ver formato abaixo)
5. Tag: git tag vX.Y.Z
6. Push: git push origin main --tags
```

**Nunca pular etapas. Nunca esquecer de incrementar a versão.**

---

## Formato de Commit Messages (COM EMOJIS)

Sempre usar o formato `<emoji> <tipo>: <descrição sucinta>`

### Tipos de Commit

| Emoji | Tipo     | Quando usar                                       |
| ----- | -------- | ------------------------------------------------- |
| ✨    | feat     | Feature nova                                      |
| 🐛    | fix      | Bug fix                                           |
| 🔧    | tweak    | Ajuste pequeno, refino, melhoria                  |
| 📝    | docs     | Alteração em README, documentação                 |
| 🎨    | style    | Mudança visual, cores, layout                     |
| ⚡    | perf     | Melhoria de performance                           |
| ♻️    | refactor | Refatoração sem mudar comportamento               |
| 📦    | build    | Mudança no build, workflow, CI/CD                 |
| 🧹    | chore    | Tarefas de manutenção, limpeza                    |

### Exemplos de Commits

```bash
# Bug fix
git commit -m "🐛 fix: correct memory display showing 0B for running containers"

# Feature nova
git commit -m "✨ feat: add install script with curl support"

# Melhoria
git commit -m "🔧 tweak: improve CPU gauge visual with filled blocks"

# Documentação
git commit -m "📝 docs: add installation instructions to README"

# Workflow/CI
git commit -m "📦 build: add GitHub Actions release workflow for x86_64 and aarch64"

# Refactor
git commit -m "♻️ refactor: simplify cgroup parsing logic"

# Performance
git commit -m "⚡ perf: reduce memory allocations in stats collection"

# Visual
git commit -m "🎨 style: apply Catppuccin Mocha theme to table borders"

# Manutenção
git commit -m "🧹 chore: remove unused dependencies from Cargo.toml"
```

---

## Regras de Versionamento (Semantic Versioning)

| Tipo de mudança      | Campo a incrementar | Exemplo               |
| -------------------- | ------------------- | --------------------- |
| Bug fix, tweak       | patch               | 0.3.3 → 0.3.4        |
| Feature nova         | minor               | 0.3.3 → 0.4.0        |
| Breaking change      | major               | 0.3.3 → 1.0.0        |

**Critérios:**
- Bug fix: corrige comportamento existente sem quebrar nada
- Feature: adiciona funcionalidade nova (flag, coluna, opção)
- Breaking: muda comportamento existente ou remove funcionalidade

---

## Como Incrementar a Versão

1. Ler `Cargo.toml` para obter a versão atual
2. Determinar o tipo de mudança (patch/minor/major)
3. Incrementar o campo apropriado
4. Salvar o `Cargo.toml`

---

## Comandos Git (sempre nesta ordem)

```bash
# 1. Stage de tudo
git add .

# 2. Commit com mensagem descritiva + emoji
git commit -m "🐛 fix: corrigir formatação de memória no output"

# 3. Criar tag
git tag vX.Y.Z

# 4. Push commit + tag
git push origin main --tags
```

**A tag `vX.Y.Z` é o que dispara o GitHub Actions workflow.**

---

## GitHub Actions Workflow

O workflow em `.github/workflows/release.yml` faz automaticamente:

1. Escuta tags no padrão `v*`
2. Compila para duas arquiteturas:
   - `x86_64-unknown-linux-musl` (ubuntu-latest)
   - `aarch64-unknown-linux-musl` (cross-compilation)
3. Cria GitHub Release com:
   - `dof-x86_64-linux`
   - `dof-aarch64-linux`

---

## Checklist (SEGUIR ESTA ORDEM)

- [ ] Modificação feita
- [ ] Versão incrementada em `Cargo.toml`
- [ ] `cargo build --release` executado com sucesso
- [ ] Commit criado com emoji + tipo + descrição sucinta
- [ ] Tag `vX.Y.Z` criada
- [ ] Push de commit + tag realizado

---

## Exemplos Completos

### Exemplo 1: Bug Fix

```bash
# Usuário pede: "corrige o bug na formatação de memória"

# 1. Corrijo o bug em src/main.rs
# 2. Incremento: 0.3.3 → 0.3.4 no Cargo.toml
# 3. Verifico:
cargo build --release

# 4. Publico:
git add .
git commit -m "🐛 fix: correct memory format showing 0B instead of actual usage"
git tag v0.3.4
git push origin main --tags
```

### Exemplo 2: Feature Nova

```bash
# Usuário pede: "adiciona flag --no-trunc"

# 1. Adiciono a flag em src/main.rs
# 2. Incremento: 0.3.3 → 0.4.0 (feature = minor)
# 3. Verifico:
cargo build --release

# 4. Publico:
git add .
git commit -m "✨ feat: add --no-trunc flag to show full container IDs"
git tag v0.4.0
git push origin main --tags
```

### Exemplo 3: Documentação

```bash
# Usuário pede: "atualiza o README com instruções de instalação"

# 1. Atualizo README.md
# 2. Incremento: 0.3.3 → 0.3.4 (docs = patch)
# 3. Publico:
git add .
git commit -m "📝 docs: add curl install instructions to README"
git tag v0.3.4
git push origin main --tags
```

### Exemplo 4: Workflow

```bash
# Usuário pede: "cria GitHub Actions para releases"

# 1. Crio .github/workflows/release.yml
# 2. Incremento: 0.3.3 → 0.4.0 (build = minor)
# 3. Publico:
git add .
git commit -m "📦 build: add GitHub Actions workflow for multi-arch releases"
git tag v0.4.0
git push origin main --tags
```
