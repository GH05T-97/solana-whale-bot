# ECR Repository
resource "aws_ecr_repository" "solana_whale_repo" {
  name                 = "solana-whale-bot"
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }
}

output "ecr_repository_url" {
  description = "The URL of the ECR repository"
  value       = aws_ecr_repository.solana_whale_repo.repository_url
}

# ECS Cluster
resource "aws_ecs_cluster" "solana_whale_cluster" {
  name = "solana-whale-cluster"

  setting {
    name  = "containerInsights"
    value = "enabled"
  }
}

# Cloudwatch Log Group
resource "aws_cloudwatch_log_group" "solana_whale_logs" {
  name              = "/ecs/solana-whale-bot"
  retention_in_days = 30
}

# ECS Task Execution Role
resource "aws_iam_role" "ecs_task_execution_role" {
  name = "solana-whale-task-execution-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "ecs-tasks.amazonaws.com"
        }
      }
    ]
  })
}

# Attach necessary policies to execution role
resource "aws_iam_role_policy_attachment" "ecs_task_execution_role_policy" {
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
  role       = aws_iam_role.ecs_task_execution_role.name
}

# ECS Task Definition
resource "aws_ecs_task_definition" "solana_whale_task" {
  family                   = "solana-whale-bot"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = 256
  memory                   = 512
  execution_role_arn       = aws_iam_role.ecs_task_execution_role.arn

  container_definitions = jsonencode([
    {
      name  = "solana-whale-bot"
      image = "${aws_ecr_repository.solana_whale_repo.repository_url}:latest"

      logConfiguration = {
        logDriver = "awslogs"
        options = {
          awslogs-group         = aws_cloudwatch_log_group.solana_whale_logs.name
          awslogs-region        = "eu-north-1"
          awslogs-stream-prefix = "ecs"
        }
      }

      secrets = [
        {
          name      = "WALLET_KEYPAIR_PATH"
          valueFrom = "arn:aws:secretsmanager:eu-north-1:211125441010:secret:solana-whale-keypair"
        },
        {
          name      = "RPC_ENDPOINT"
          valueFrom = "arn:aws:secretsmanager:eu-north-1:211125441010:secret:solana-rpc-endpoint"
        }
      ]
    }
  ])
}

# ECS Service
resource "aws_ecs_service" "solana_whale_service" {
  name            = "solana-whale-bot-service"
  cluster         = aws_ecs_cluster.solana_whale_cluster.id
  task_definition = aws_ecs_task_definition.solana_whale_task.arn
  launch_type     = "FARGATE"
  desired_count   = 1

  network_configuration {
    subnets         = aws_subnet.private_subnets[*].id
    security_groups = [aws_security_group.ecs_tasks.id]
  }
}

# Security Group for ECS Tasks
resource "aws_security_group" "ecs_tasks" {
  name        = "solana-whale-ecs-tasks-sg"
  description = "Allow outbound traffic for ECS tasks"
  vpc_id      = aws_vpc.solana_whale_vpc.id

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}