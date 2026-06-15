---
name: dof-release
description: >
  Use SEMPRE que houver qualquer modificação nos arquivos src/ deste projeto.
  Esta skill controla o ciclo de versionamento e release do projeto dof.
  Toda alteração em src/ DEVE incrementar a versão no Cargo.toml,
  criar commit, tag e push para disparar a GitHub Release automaticamente.
---

# dof-release — Versionamento e Release

## Regra Inviolável

**Toda modificação em `src/` DEVE resultar em uma nova versão publicada.**

A fonte da verdade da versão é `Cargo.toml` → campo `version`.

---

## Fluxo Obrigatório

Quando o usuário pedir para alterar qualquer coisa nos fontes:

```
1. Fazer a modificação em src/
2. Incrementar versão no Cargo.toml
3. Build e verificação: cargo build --release
4. Commit: git add . && git commit -m "release: vX.Y.Z"
5. Tag: git tag vX.Y.Z
6. Push: git push origin main --tags
```

**Nunca pular etapas. Nunca esquecer de incrementar a versão.**

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

Exemplo de incremento patch:
```toml
# Antes
version = "0.3.3"

# Depois
version = "0.3.4"
```

---

## Comandos Git (sempre nesta ordem)

```bash
# 1. Stage de tudo
git add .

# 2. Commit com mensagem padronizada
git commit -m "release: vX.Y.Z"

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
   - `x86_64-unknown-linux-gnu` (ubuntu-latest)
   - `aarch64-unknown-linux-gnu` (ubuntu-24.04-arm)
3. Cria GitHub Release com:
   - `dof-linux-x86_64`
   - `dof-linux-aarch64`
   - `sha256sums.txt`

---

## Checklist (SEGUIR ESTA ORDEM)

- [ ] Modificação feita em `src/`
- [ ] Versão incrementada em `Cargo.toml`
- [ ] `cargo build --release` executado com sucesso
- [ ] Commit criado com mensagem `release: vX.Y.Z`
- [ ] Tag `vX.Y.Z` criada
- [ ] Push de commit + tag realizado

---

## Exemplo Completo (Bug Fix)

```bash
# Usuário pede: "corrige o bug na formatação de memória"

# 1. Eu faço a correção em src/main.rs
# 2. Incremento: 0.3.3 → 0.3.4 no Cargo.toml
# 3. Verifico:
cargo build --release

# 4. Publico:
git add .
git commit -m "release: v0.3.4"
git tag v0.3.4
git push origin main --tags
```
