# Repository A - Uses newer AWS provider version

terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = ">= 5.0"
    }
  }
}

module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = ">= 5.0"

  name = "vpc-a"
  cidr = "10.0.0.0/16"
}

