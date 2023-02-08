// AWS S3 Configuration
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::model::{BucketLocationConstraint, CreateBucketConfiguration};
use aws_sdk_s3::types::ByteStream;
use aws_sdk_s3::{Client, Error};
use std::path::Path;
use std::process;

// Determine AWS region
pub async fn bucket_region() -> Result<String, Error> {
    let region_provider = RegionProviderChain::first_try(None)
        .or_default_provider()
        .or_else("us-west-2");
    let region = region_provider.region().await.unwrap();
    Ok(region.to_string())
}

// Create AWS client
pub async fn client() -> Result<Client, Error> {
    let region_provider = RegionProviderChain::first_try(None)
        .or_default_provider()
        .or_else("us-west-2");
    let shared_config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&shared_config);
    Ok(client)
}

/* -----------------------------
    BUCKET FNXNS
-------------------------------- */

// List all buckets
pub async fn list_buckets(client: &Client) -> Result<(), Error> {
    let resp = client.list_buckets().send().await?;
    let buckets = resp.buckets().unwrap_or_default();
    let num_buckets = buckets.len();
    println!("Found {} buckets.", num_buckets);
    println!();
    for bucket in buckets {
        println!("{}", bucket.name().unwrap_or_default());
    }

    Ok(())
}

// Check if bucket exists
pub async fn bucket_exists(client: &Client, bucket_name: &str) -> Result<bool, Error> {
    let resp = client.list_buckets().send().await?;
    let buckets = resp.buckets().unwrap_or_default();
    for bucket in buckets {
        if bucket.name().unwrap_or_default() == bucket_name {
            return Ok(true);
        }
    }
    Ok(false)
}

// Create new bucket
pub async fn create_bucket(client: &Client, bucket: &str, region: &str) -> Result<(), Error> {
    // Check if bucket exists
    let exists = bucket_exists(client, bucket).await?;
    if exists {
        println!("Bucket {bucket} already exists.");
        process::exit(1);
    }
    let constraint = BucketLocationConstraint::from(region);
    let cfg = CreateBucketConfiguration::builder()
        .location_constraint(constraint)
        .build();
    client
        .create_bucket()
        .create_bucket_configuration(cfg)
        .bucket(bucket)
        .send()
        .await?;
    println!("Creating bucket named: {bucket} in region: {region}");
    Ok(())
}

// Delete empty bucket
pub async fn delete_bucket(client: &Client, bucket: &str) -> Result<(), Error> {
    let exists = bucket_exists(client, bucket).await?;
    if !exists {
        println!("Bucket {bucket} does not exist.");
        process::exit(1);
    }
    let resp = client.list_objects_v2().bucket(bucket).send().await?;
    let objects = resp.contents().unwrap_or_default();
    let num_objects = objects.len();
    if num_objects != 0 {
        println!("Bucket {bucket} is not empty. Cannot delete.");
        process::exit(1);
    }
    client.delete_bucket().bucket(bucket).send().await?;
    println!("Empty bucket {bucket} deleted.");

    Ok(())
}

/* -----------------------------
    OBJECT FNXNS
--------------------------------*/

// List objects in bucket
pub async fn list_objects(client: &Client, bucket: &str) -> Result<(), Error> {
    // Check if bucket exists
    let exists = bucket_exists(client, bucket).await?;
    if !exists {
        println!("Bucket {bucket} does not exist.");
        process::exit(1);
    }

    // If exists, list objects
    let resp = client.list_objects_v2().bucket(bucket).send().await?;
    let objects = resp.contents().unwrap_or_default();
    let num_objects = objects.len();

    println!("Found {num_objects} objects in bucket {bucket}");
    for object in objects {
        println!("{}", object.key().unwrap_or_default());
    }

    Ok(())
}

// Put object in bucket
pub async fn upload_object(client: &Client, bucket: &str, filepath: &str) -> Result<(), Error> {
    // if bucket doesn't exist, create it
    if !bucket_exists(client, bucket).await? {
        let bucket_region = bucket_region().await.unwrap();
        create_bucket(client, bucket, &bucket_region).await?;
    }

    let body = ByteStream::from_path(Path::new(filepath)).await;
    let key = Path::new(filepath).file_name().unwrap().to_str().unwrap();
    match body {
        Ok(b) => {
            let _resp = client
                .put_object()
                .bucket(bucket)
                .key(key)
                .body(b)
                .send()
                .await?;
            println!("Uploaded {key} to {bucket}");
        }
        Err(e) => {
            println!("Got an error uploading object:");
            println!("{}", e);
            process::exit(1);
        }
    }

    Ok(())
}

// Delete object from bucket
pub async fn delete_object(client: &Client, bucket: &str, key: &str) -> Result<(), Error> {
    // Check if bucket exists
    let exists = bucket_exists(client, bucket).await?;
    if !exists {
        println!("Bucket {bucket} does not exist.");
        process::exit(1);
    }

    // Check key exists in bucket
    let resp = client.list_objects_v2().bucket(bucket).send().await?;
    let objects = resp.contents().unwrap_or_default();
    let mut key_exists = false;
    for object in objects {
        if object.key().unwrap_or_default() == key {
            key_exists = true;
        }
    }
    if !key_exists {
        println!("Key {key} does not exist in bucket {bucket}");
        process::exit(1);
    }
    // Delete object
    client
        .delete_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;

    println!("Object {key} deleted from bucket {bucket}.");

    Ok(())
}
