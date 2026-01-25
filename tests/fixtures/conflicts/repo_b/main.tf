# Repository B - Uses older AWS provider version (CONFLICT!)

terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "<= 4.5"
    }
  }
}

module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "<= 4.0"

  name = "vpc-b"
  cidr = "10.1.0.0/16"
}

