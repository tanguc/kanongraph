# Configuration with risky patterns for testing

terraform {
  required_providers {
    # No version constraint - risky!
    aws = {
      source = "hashicorp/aws"
    }
    
    # Overly broad constraint
    google = {
      source  = "hashicorp/google"
      version = ">= 0.0.0"
    }
    
    # No upper bound
    azurerm = {
      source  = "hashicorp/azurerm"
      version = ">= 3.0"
    }
  }
}

# Module without version constraint
module "vpc_no_version" {
  source = "terraform-aws-modules/vpc/aws"

  name = "no-version-vpc"
  cidr = "10.0.0.0/16"
}

# Module with exact version (no flexibility)
module "eks_exact" {
  source  = "terraform-aws-modules/eks/aws"
  version = "19.15.3"

  cluster_name = "exact-version-cluster"
}

# Local module reference
module "local_module" {
  source = "../modules/custom"

  name = "local"
}

# Git module
module "git_module" {
  source = "git::https://github.com/example/terraform-module.git?ref=v1.0.0"

  name = "git-sourced"
}

