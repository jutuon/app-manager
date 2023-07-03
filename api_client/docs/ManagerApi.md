# \ManagerApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_encryption_key**](ManagerApi.md#get_encryption_key) | **GET** /manager_api/encryption_key/{server} | Get encryption key for some server



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

