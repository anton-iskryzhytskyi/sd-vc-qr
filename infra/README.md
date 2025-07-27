# INFRA

## How to execute benches on amazon EC2

It is important to use this configuration of instance type and this configuration of volume size, as with lower configuration benches may not run.

1. Launch ec2 instance with arch - arm64,instance type - t4g.xlarge, OS - Amazon Linux 2023, Volume size 32 GB
2. Connect via SSH and install Docker and Git. Execute one by one.
  ```
  sudo dnf update -y 
  sudo dnf install -y git
  sudo dnf install --allowerasing -y \
  docker \
  containerd \
  runc \
  container-selinux \
  cni-plugins \
  oci-add-hooks \
  amazon-ecr-credential-helper \
  udica

  sudo groupadd docker
  sudo usermod -aG docker $USER
  newgrp docker 

  sudo systemctl enable --now docker.service containerd.service
  sudo systemctl status docker

  sudo mkdir -p /usr/local/lib/docker/cli-plugins
  sudo curl -sL \
  https://github.com/docker/compose/releases/latest/download/docker-compose-linux-$(uname -m) \
  -o /usr/local/lib/docker/cli-plugins/docker-compose
  
  sudo chmod +x /usr/local/lib/docker/cli-plugins/docker-compose
  ```

3. clone repo
  ```
  git clone https://github.com/anton-iskryzhytskyi/sd-vc-qr.git
  ```
4. run benches and data collection
  ```
  cd sd-vc-qr/infra
  export DOCKER_BUILDKIT=0
  docker compose up --build
  ```
5. Download artifacts (benchmark results) for review with scp.