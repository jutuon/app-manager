# \ManagerApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_encryption_key**](ManagerApi.md#get_encryption_key) | **GET** /manager_api/encryption_key/{server} | Get encryption key for some server
[**get_latest_software**](ManagerApi.md#get_latest_software) | **GET** /manager_api/latest_software | Download latest software
[**get_software_info**](ManagerApi.md#get_software_info) | **GET** /manager_api/software_info | Get current software info about currently installed backend and manager.
[**post_request_build_software**](ManagerApi.md#post_request_build_software) | **POST** /manager_api/request_build_software | Request building the latest software from git.
[**post_request_software_update**](ManagerApi.md#post_request_software_update) | **POST** /manager_api/request_software_update | Request software update.



## get_encryption_key

> crate::models::DataEncryptionKey get_encryption_key(server)
Get encryption key for some server

Get encryption key for some server

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**server** | **String** |  | [required] |

### Return type

[**crate::models::DataEncryptionKey**](DataEncryptionKey.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_latest_software

> std::path::PathBuf get_latest_software(software_options, download_type)
Download latest software

Download latest software

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**software_options** | [**SoftwareOptions**](.md) |  | [required] |
**download_type** | [**DownloadType**](.md) |  | [required] |

### Return type

[**std::path::PathBuf**](std::path::PathBuf.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/octet-stream

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_software_info

> crate::models::SoftwareInfo get_software_info()
Get current software info about currently installed backend and manager.

Get current software info about currently installed backend and manager.

### Parameters

This endpoint does not need any parameter.

### Return type

[**crate::models::SoftwareInfo**](SoftwareInfo.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## post_request_build_software

> post_request_build_software(software_options)
Request building the latest software from git.

Request building the latest software from git.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**software_options** | [**SoftwareOptions**](.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## post_request_software_update

> post_request_software_update(software_options, reboot)
Request software update.

Request software update.  Manager will update the requested software and reboot the computer as soon as possible if specified.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**software_options** | [**SoftwareOptions**](.md) |  | [required] |
**reboot** | **bool** |  | [required] |

### Return type

 (empty response body)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

