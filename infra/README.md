# INFRA

## How to execute benches on amazon EC2

1. Launch ec2 instance with arch - arm64,instance type - t4g.xlarge, OS - Amazon Linux 2023
2. Connect via SSH and install Docker and Git.
  ```
  sudo dnf update -y 
  sudo dnf install -y docker git
  ```
3. Start docker and allow ec2-user to invoke it w/o `sudo`
  ```
  sudo systemctl enable docker
  sudo systemctl start docker
  ```
4. Verify
  ```
  sudo docker --version  
  git --version
  ```