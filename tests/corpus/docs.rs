//! Real-world YAML documents used by the shared corpus.
//!
//! Each constant is a self-contained, realistic document drawn from a common
//! YAML use case (Kubernetes, GitHub Actions, Docker Compose, Helm, app
//! config). They are deliberately verbatim — comments, quoting, anchors, block
//! scalars, and multi-document markers are preserved so the fidelity engines
//! can be exercised on genuine formatting, not toy input.

// yqr-m003: shared validation + benchmark corpus (real-world documents).

/// A Kubernetes Deployment manifest: nested metadata, labels with `/`-and-`.`
/// keys, a container list with ports, env, and resource requests.
pub const K8S_DEPLOYMENT: &str = "\
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web
  namespace: production
  labels:
    app.kubernetes.io/name: web
    app.kubernetes.io/component: frontend
spec:
  replicas: 3
  selector:
    matchLabels:
      app: web
  template:
    metadata:
      labels:
        app: web
    spec:
      containers:
        - name: web
          image: registry.example.com/web:1.4.2
          ports:
            - containerPort: 8080
              protocol: TCP
          env:
            - name: LOG_LEVEL
              value: info
            - name: TIMEOUT
              value: \"30\"
          resources:
            requests:
              cpu: 250m
              memory: 256Mi
            limits:
              cpu: \"1\"
              memory: 512Mi
        - name: sidecar
          image: registry.example.com/proxy:0.9.0
          ports:
            - containerPort: 9090
              protocol: TCP
";

/// A GitHub Actions workflow: `on` triggers, a build job with a step list and
/// a strategy matrix.
pub const GH_ACTIONS: &str = "\
name: CI
on:
  push:
    branches:
      - main
  pull_request: {}
jobs:
  build:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        rust:
          - \"1.96\"
          - stable
    steps:
      - name: Checkout
        uses: actions/checkout@v5
      - name: Build
        run: cargo build --release
      - name: Test
        run: cargo test --all-features
";

/// A Docker Compose file: two services, published ports, an environment map,
/// named volumes, and a dependency edge.
pub const DOCKER_COMPOSE: &str = "\
version: \"3.9\"
services:
  web:
    image: nginx:1.27
    ports:
      - \"80:80\"
      - \"443:443\"
    depends_on:
      - db
    environment:
      NGINX_HOST: example.com
      NGINX_PORT: \"80\"
  db:
    image: postgres:16
    volumes:
      - pgdata:/var/lib/postgresql/data
    environment:
      POSTGRES_PASSWORD: secret
volumes:
  pgdata: {}
";

/// A Helm `values.yaml`: scalars, nested maps, a host list, and resource
/// limits/requests.
pub const HELM_VALUES: &str = "\
replicaCount: 2
image:
  repository: registry.example.com/api
  tag: 2.1.0
  pullPolicy: IfNotPresent
service:
  type: ClusterIP
  port: 8080
ingress:
  enabled: true
  hosts:
    - host: api.example.com
      paths:
        - /
resources:
  limits:
    cpu: 500m
    memory: 512Mi
  requests:
    cpu: 100m
    memory: 128Mi
";

/// An application config: server binding, database pool, a feature-flag list,
/// and logging. Uses a quoted numeric string to exercise scalar spelling.
pub const APP_CONFIG: &str = "\
server:
  host: 0.0.0.0
  port: 8443
database:
  url: postgres://localhost/app
  pool:
    min: 2
    max: 16
features:
  - search
  - export
  - audit-log
logging:
  level: warn
  format: json
zip: \"007\"
";

/// A multi-document stream: a ConfigMap followed by a Service. Exercises the
/// classic pipeline's first-document semantics and the engines' per-document
/// evaluation.
pub const MULTI_DOC: &str = "\
apiVersion: v1
kind: ConfigMap
metadata:
  name: settings
data:
  LOG_LEVEL: info
---
apiVersion: v1
kind: Service
metadata:
  name: web
spec:
  selector:
    app: web
  ports:
    - port: 80
      targetPort: 8080
";

/// A document rich in formatting the classic pipeline would normalize away:
/// a leading comment, an inline comment, an anchor/alias pair, a literal block
/// scalar, and single-quoted scalars. Used to prove the fidelity engines keep
/// these bytes verbatim.
pub const FIDELITY_RICH: &str = "\
# deployment defaults
defaults: &defaults
  retries: 3
  timeout: 30      # seconds
service:
  <<: *defaults
  name: 'web-frontend'
  motd: |
    welcome
    to the service
  region: 'us-east-1'
";

/// Build a large inventory document with `n` host records — used by the
/// benchmark to measure iteration/projection at scale. Only the benchmark crate
/// consumes it, so it is dead code from the validation crate's point of view.
#[must_use]
#[allow(dead_code)]
pub fn inventory(n: usize) -> String {
    let mut s = String::from("hosts:\n");
    for i in 0..n {
        s.push_str(&format!(
            "  - name: host-{i}\n    ip: 10.0.{}.{}\n    role: {}\n    port: {}\n",
            i / 256,
            i % 256,
            if i % 3 == 0 { "leader" } else { "follower" },
            8000 + (i % 1000),
        ));
    }
    s
}
