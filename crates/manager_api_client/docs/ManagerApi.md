# \ManagerApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_encryption_key**](ManagerApi.md#get_encryption_key) | **GET** /manager_api/encryption_key/{server} | Get encryption key for some server
[**get_latest_software**](ManagerApi.md#get_latest_software) | **GET** /manager_api/latest_software | Download latest software.
[**get_software_info**](ManagerApi.md#get_software_info) | **GET** /manager_api/software_info | Get current software info about currently installed backend and manager.
[**get_system_info**](ManagerApi.md#get_system_info) | **GET** /manager_api/system_info | Get system info about current operating system, hardware and software.
[**get_system_info_all**](ManagerApi.md#get_system_info_all) | **GET** /manager_api/system_info_all | Get system info about current operating system, hardware and software.
[**post_request_build_software**](ManagerApi.md#post_request_build_software) | **POST** /manager_api/request_build_software | Request building the latest software from git.
[**post_request_restart_or_reset_backend**](ManagerApi.md#post_request_restart_or_reset_backend) | **POST** /manager_api/request_restart_or_reset_backend | Restart or reset backend.
[**post_request_software_update**](ManagerApi.md#post_request_software_update) | **POST** /manager_api/request_software_update | Request software update.



## get_encryption_key

> models::DataEncryptionKey get_encryption_key(server)
Get encryption key for some server

Get encryption key for some server

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**server** | **String** |  | [required] |

### Return type

[**models::DataEncryptionKey**](DataEncryptionKey.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_latest_software

> std::path::PathBuf get_latest_software(software_options, download_type)
Download latest software.

Download latest software.  Returns BuildInfo JSON or encrypted binary depending on DownloadTypeQueryParam value.

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

> models::SoftwareInfo get_software_info()
Get current software info about currently installed backend and manager.

Get current software info about currently installed backend and manager.

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::SoftwareInfo**](SoftwareInfo.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_system_info

> models::SystemInfo get_system_info()
Get system info about current operating system, hardware and software.

Get system info about current operating system, hardware and software.  Returns system info related to current manager instance.

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::SystemInfo**](SystemInfo.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_system_info_all

> models::SystemInfoList get_system_info_all()
Get system info about current operating system, hardware and software.

Get system info about current operating system, hardware and software.  Returns system info related to current manager instance and ones defined in config file.

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::SystemInfoList**](SystemInfoList.md)

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


## post_request_restart_or_reset_backend

> post_request_restart_or_reset_backend(reset_data)
Restart or reset backend.

Restart or reset backend.  Restarts backend process. Optionally backend data storage can be reset also. The data reset will work as described in request_software_update request documentation.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**reset_data** | **bool** |  | [required] |

### Return type

 (empty response body)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## post_request_software_update

> post_request_software_update(software_options, reboot, reset_data)
Request software update.

Request software update.  Manager will update the requested software and reboot the computer as soon as possible if specified.  Software's current data storage can be resetted. This will move the data in the data storage to another location waiting for deletion. The deletetion will happen when the next data reset happens. The selected software must support data reset_data query parameter. Resetting the data storage can only work if it is configured from app-manager config file.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**software_options** | [**SoftwareOptions**](.md) |  | [required] |
**reboot** | **bool** |  | [required] |
**reset_data** | **bool** |  | [required] |

### Return type

 (empty response body)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

