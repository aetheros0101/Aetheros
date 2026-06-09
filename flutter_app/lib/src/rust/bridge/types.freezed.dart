// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'types.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

T _$identity<T>(T value) => value;

final _privateConstructorUsedError = UnsupportedError(
    'It seems like you constructed your class using `MyClass._()`. This constructor is only meant to be used by freezed and you are not supposed to need it nor use it.\nPlease check the documentation here for more information: https://github.com/rrousselGit/freezed#adding-getters-and-methods-to-our-models');

/// @nodoc
mixin _$ModuleUploadResponse {
  String get hash => throw _privateConstructorUsedError;
  BigInt get size => throw _privateConstructorUsedError;

  /// Create a copy of ModuleUploadResponse
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  $ModuleUploadResponseCopyWith<ModuleUploadResponse> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $ModuleUploadResponseCopyWith<$Res> {
  factory $ModuleUploadResponseCopyWith(ModuleUploadResponse value,
          $Res Function(ModuleUploadResponse) then) =
      _$ModuleUploadResponseCopyWithImpl<$Res, ModuleUploadResponse>;
  @useResult
  $Res call({String hash, BigInt size});
}

/// @nodoc
class _$ModuleUploadResponseCopyWithImpl<$Res,
        $Val extends ModuleUploadResponse>
    implements $ModuleUploadResponseCopyWith<$Res> {
  _$ModuleUploadResponseCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of ModuleUploadResponse
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? hash = null,
    Object? size = null,
  }) {
    return _then(_value.copyWith(
      hash: null == hash
          ? _value.hash
          : hash // ignore: cast_nullable_to_non_nullable
              as String,
      size: null == size
          ? _value.size
          : size // ignore: cast_nullable_to_non_nullable
              as BigInt,
    ) as $Val);
  }
}

/// @nodoc
abstract class _$$ModuleUploadResponseImplCopyWith<$Res>
    implements $ModuleUploadResponseCopyWith<$Res> {
  factory _$$ModuleUploadResponseImplCopyWith(_$ModuleUploadResponseImpl value,
          $Res Function(_$ModuleUploadResponseImpl) then) =
      __$$ModuleUploadResponseImplCopyWithImpl<$Res>;
  @override
  @useResult
  $Res call({String hash, BigInt size});
}

/// @nodoc
class __$$ModuleUploadResponseImplCopyWithImpl<$Res>
    extends _$ModuleUploadResponseCopyWithImpl<$Res, _$ModuleUploadResponseImpl>
    implements _$$ModuleUploadResponseImplCopyWith<$Res> {
  __$$ModuleUploadResponseImplCopyWithImpl(_$ModuleUploadResponseImpl _value,
      $Res Function(_$ModuleUploadResponseImpl) _then)
      : super(_value, _then);

  /// Create a copy of ModuleUploadResponse
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? hash = null,
    Object? size = null,
  }) {
    return _then(_$ModuleUploadResponseImpl(
      hash: null == hash
          ? _value.hash
          : hash // ignore: cast_nullable_to_non_nullable
              as String,
      size: null == size
          ? _value.size
          : size // ignore: cast_nullable_to_non_nullable
              as BigInt,
    ));
  }
}

/// @nodoc

class _$ModuleUploadResponseImpl implements _ModuleUploadResponse {
  const _$ModuleUploadResponseImpl({required this.hash, required this.size});

  @override
  final String hash;
  @override
  final BigInt size;

  @override
  String toString() {
    return 'ModuleUploadResponse(hash: $hash, size: $size)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$ModuleUploadResponseImpl &&
            (identical(other.hash, hash) || other.hash == hash) &&
            (identical(other.size, size) || other.size == size));
  }

  @override
  int get hashCode => Object.hash(runtimeType, hash, size);

  /// Create a copy of ModuleUploadResponse
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$ModuleUploadResponseImplCopyWith<_$ModuleUploadResponseImpl>
      get copyWith =>
          __$$ModuleUploadResponseImplCopyWithImpl<_$ModuleUploadResponseImpl>(
              this, _$identity);
}

abstract class _ModuleUploadResponse implements ModuleUploadResponse {
  const factory _ModuleUploadResponse(
      {required final String hash,
      required final BigInt size}) = _$ModuleUploadResponseImpl;

  @override
  String get hash;
  @override
  BigInt get size;

  /// Create a copy of ModuleUploadResponse
  /// with the given fields replaced by the non-null parameter values.
  @override
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$ModuleUploadResponseImplCopyWith<_$ModuleUploadResponseImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$TaskRequest {
  String get wasmModuleHash => throw _privateConstructorUsedError;
  String get entrypoint => throw _privateConstructorUsedError;
  String get priority => throw _privateConstructorUsedError;
  BigInt get timeoutMs => throw _privateConstructorUsedError;
  int get maxRetries => throw _privateConstructorUsedError;

  /// Create a copy of TaskRequest
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  $TaskRequestCopyWith<TaskRequest> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $TaskRequestCopyWith<$Res> {
  factory $TaskRequestCopyWith(
          TaskRequest value, $Res Function(TaskRequest) then) =
      _$TaskRequestCopyWithImpl<$Res, TaskRequest>;
  @useResult
  $Res call(
      {String wasmModuleHash,
      String entrypoint,
      String priority,
      BigInt timeoutMs,
      int maxRetries});
}

/// @nodoc
class _$TaskRequestCopyWithImpl<$Res, $Val extends TaskRequest>
    implements $TaskRequestCopyWith<$Res> {
  _$TaskRequestCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of TaskRequest
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? wasmModuleHash = null,
    Object? entrypoint = null,
    Object? priority = null,
    Object? timeoutMs = null,
    Object? maxRetries = null,
  }) {
    return _then(_value.copyWith(
      wasmModuleHash: null == wasmModuleHash
          ? _value.wasmModuleHash
          : wasmModuleHash // ignore: cast_nullable_to_non_nullable
              as String,
      entrypoint: null == entrypoint
          ? _value.entrypoint
          : entrypoint // ignore: cast_nullable_to_non_nullable
              as String,
      priority: null == priority
          ? _value.priority
          : priority // ignore: cast_nullable_to_non_nullable
              as String,
      timeoutMs: null == timeoutMs
          ? _value.timeoutMs
          : timeoutMs // ignore: cast_nullable_to_non_nullable
              as BigInt,
      maxRetries: null == maxRetries
          ? _value.maxRetries
          : maxRetries // ignore: cast_nullable_to_non_nullable
              as int,
    ) as $Val);
  }
}

/// @nodoc
abstract class _$$TaskRequestImplCopyWith<$Res>
    implements $TaskRequestCopyWith<$Res> {
  factory _$$TaskRequestImplCopyWith(
          _$TaskRequestImpl value, $Res Function(_$TaskRequestImpl) then) =
      __$$TaskRequestImplCopyWithImpl<$Res>;
  @override
  @useResult
  $Res call(
      {String wasmModuleHash,
      String entrypoint,
      String priority,
      BigInt timeoutMs,
      int maxRetries});
}

/// @nodoc
class __$$TaskRequestImplCopyWithImpl<$Res>
    extends _$TaskRequestCopyWithImpl<$Res, _$TaskRequestImpl>
    implements _$$TaskRequestImplCopyWith<$Res> {
  __$$TaskRequestImplCopyWithImpl(
      _$TaskRequestImpl _value, $Res Function(_$TaskRequestImpl) _then)
      : super(_value, _then);

  /// Create a copy of TaskRequest
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? wasmModuleHash = null,
    Object? entrypoint = null,
    Object? priority = null,
    Object? timeoutMs = null,
    Object? maxRetries = null,
  }) {
    return _then(_$TaskRequestImpl(
      wasmModuleHash: null == wasmModuleHash
          ? _value.wasmModuleHash
          : wasmModuleHash // ignore: cast_nullable_to_non_nullable
              as String,
      entrypoint: null == entrypoint
          ? _value.entrypoint
          : entrypoint // ignore: cast_nullable_to_non_nullable
              as String,
      priority: null == priority
          ? _value.priority
          : priority // ignore: cast_nullable_to_non_nullable
              as String,
      timeoutMs: null == timeoutMs
          ? _value.timeoutMs
          : timeoutMs // ignore: cast_nullable_to_non_nullable
              as BigInt,
      maxRetries: null == maxRetries
          ? _value.maxRetries
          : maxRetries // ignore: cast_nullable_to_non_nullable
              as int,
    ));
  }
}

/// @nodoc

class _$TaskRequestImpl implements _TaskRequest {
  const _$TaskRequestImpl(
      {required this.wasmModuleHash,
      required this.entrypoint,
      required this.priority,
      required this.timeoutMs,
      required this.maxRetries});

  @override
  final String wasmModuleHash;
  @override
  final String entrypoint;
  @override
  final String priority;
  @override
  final BigInt timeoutMs;
  @override
  final int maxRetries;

  @override
  String toString() {
    return 'TaskRequest(wasmModuleHash: $wasmModuleHash, entrypoint: $entrypoint, priority: $priority, timeoutMs: $timeoutMs, maxRetries: $maxRetries)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$TaskRequestImpl &&
            (identical(other.wasmModuleHash, wasmModuleHash) ||
                other.wasmModuleHash == wasmModuleHash) &&
            (identical(other.entrypoint, entrypoint) ||
                other.entrypoint == entrypoint) &&
            (identical(other.priority, priority) ||
                other.priority == priority) &&
            (identical(other.timeoutMs, timeoutMs) ||
                other.timeoutMs == timeoutMs) &&
            (identical(other.maxRetries, maxRetries) ||
                other.maxRetries == maxRetries));
  }

  @override
  int get hashCode => Object.hash(
      runtimeType, wasmModuleHash, entrypoint, priority, timeoutMs, maxRetries);

  /// Create a copy of TaskRequest
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$TaskRequestImplCopyWith<_$TaskRequestImpl> get copyWith =>
      __$$TaskRequestImplCopyWithImpl<_$TaskRequestImpl>(this, _$identity);
}

abstract class _TaskRequest implements TaskRequest {
  const factory _TaskRequest(
      {required final String wasmModuleHash,
      required final String entrypoint,
      required final String priority,
      required final BigInt timeoutMs,
      required final int maxRetries}) = _$TaskRequestImpl;

  @override
  String get wasmModuleHash;
  @override
  String get entrypoint;
  @override
  String get priority;
  @override
  BigInt get timeoutMs;
  @override
  int get maxRetries;

  /// Create a copy of TaskRequest
  /// with the given fields replaced by the non-null parameter values.
  @override
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$TaskRequestImplCopyWith<_$TaskRequestImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$TaskStatusResponse {
  String get taskId => throw _privateConstructorUsedError;
  String get state => throw _privateConstructorUsedError;
  int get createdAt => throw _privateConstructorUsedError;
  int get updatedAt => throw _privateConstructorUsedError;
  int get attempts => throw _privateConstructorUsedError;
  String? get errorMessage => throw _privateConstructorUsedError;

  /// Create a copy of TaskStatusResponse
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  $TaskStatusResponseCopyWith<TaskStatusResponse> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $TaskStatusResponseCopyWith<$Res> {
  factory $TaskStatusResponseCopyWith(
          TaskStatusResponse value, $Res Function(TaskStatusResponse) then) =
      _$TaskStatusResponseCopyWithImpl<$Res, TaskStatusResponse>;
  @useResult
  $Res call(
      {String taskId,
      String state,
      int createdAt,
      int updatedAt,
      int attempts,
      String? errorMessage});
}

/// @nodoc
class _$TaskStatusResponseCopyWithImpl<$Res, $Val extends TaskStatusResponse>
    implements $TaskStatusResponseCopyWith<$Res> {
  _$TaskStatusResponseCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of TaskStatusResponse
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? taskId = null,
    Object? state = null,
    Object? createdAt = null,
    Object? updatedAt = null,
    Object? attempts = null,
    Object? errorMessage = freezed,
  }) {
    return _then(_value.copyWith(
      taskId: null == taskId
          ? _value.taskId
          : taskId // ignore: cast_nullable_to_non_nullable
              as String,
      state: null == state
          ? _value.state
          : state // ignore: cast_nullable_to_non_nullable
              as String,
      createdAt: null == createdAt
          ? _value.createdAt
          : createdAt // ignore: cast_nullable_to_non_nullable
              as int,
      updatedAt: null == updatedAt
          ? _value.updatedAt
          : updatedAt // ignore: cast_nullable_to_non_nullable
              as int,
      attempts: null == attempts
          ? _value.attempts
          : attempts // ignore: cast_nullable_to_non_nullable
              as int,
      errorMessage: freezed == errorMessage
          ? _value.errorMessage
          : errorMessage // ignore: cast_nullable_to_non_nullable
              as String?,
    ) as $Val);
  }
}

/// @nodoc
abstract class _$$TaskStatusResponseImplCopyWith<$Res>
    implements $TaskStatusResponseCopyWith<$Res> {
  factory _$$TaskStatusResponseImplCopyWith(_$TaskStatusResponseImpl value,
          $Res Function(_$TaskStatusResponseImpl) then) =
      __$$TaskStatusResponseImplCopyWithImpl<$Res>;
  @override
  @useResult
  $Res call(
      {String taskId,
      String state,
      int createdAt,
      int updatedAt,
      int attempts,
      String? errorMessage});
}

/// @nodoc
class __$$TaskStatusResponseImplCopyWithImpl<$Res>
    extends _$TaskStatusResponseCopyWithImpl<$Res, _$TaskStatusResponseImpl>
    implements _$$TaskStatusResponseImplCopyWith<$Res> {
  __$$TaskStatusResponseImplCopyWithImpl(_$TaskStatusResponseImpl _value,
      $Res Function(_$TaskStatusResponseImpl) _then)
      : super(_value, _then);

  /// Create a copy of TaskStatusResponse
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? taskId = null,
    Object? state = null,
    Object? createdAt = null,
    Object? updatedAt = null,
    Object? attempts = null,
    Object? errorMessage = freezed,
  }) {
    return _then(_$TaskStatusResponseImpl(
      taskId: null == taskId
          ? _value.taskId
          : taskId // ignore: cast_nullable_to_non_nullable
              as String,
      state: null == state
          ? _value.state
          : state // ignore: cast_nullable_to_non_nullable
              as String,
      createdAt: null == createdAt
          ? _value.createdAt
          : createdAt // ignore: cast_nullable_to_non_nullable
              as int,
      updatedAt: null == updatedAt
          ? _value.updatedAt
          : updatedAt // ignore: cast_nullable_to_non_nullable
              as int,
      attempts: null == attempts
          ? _value.attempts
          : attempts // ignore: cast_nullable_to_non_nullable
              as int,
      errorMessage: freezed == errorMessage
          ? _value.errorMessage
          : errorMessage // ignore: cast_nullable_to_non_nullable
              as String?,
    ));
  }
}

/// @nodoc

class _$TaskStatusResponseImpl implements _TaskStatusResponse {
  const _$TaskStatusResponseImpl(
      {required this.taskId,
      required this.state,
      required this.createdAt,
      required this.updatedAt,
      required this.attempts,
      this.errorMessage});

  @override
  final String taskId;
  @override
  final String state;
  @override
  final int createdAt;
  @override
  final int updatedAt;
  @override
  final int attempts;
  @override
  final String? errorMessage;

  @override
  String toString() {
    return 'TaskStatusResponse(taskId: $taskId, state: $state, createdAt: $createdAt, updatedAt: $updatedAt, attempts: $attempts, errorMessage: $errorMessage)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$TaskStatusResponseImpl &&
            (identical(other.taskId, taskId) || other.taskId == taskId) &&
            (identical(other.state, state) || other.state == state) &&
            (identical(other.createdAt, createdAt) ||
                other.createdAt == createdAt) &&
            (identical(other.updatedAt, updatedAt) ||
                other.updatedAt == updatedAt) &&
            (identical(other.attempts, attempts) ||
                other.attempts == attempts) &&
            (identical(other.errorMessage, errorMessage) ||
                other.errorMessage == errorMessage));
  }

  @override
  int get hashCode => Object.hash(
      runtimeType, taskId, state, createdAt, updatedAt, attempts, errorMessage);

  /// Create a copy of TaskStatusResponse
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$TaskStatusResponseImplCopyWith<_$TaskStatusResponseImpl> get copyWith =>
      __$$TaskStatusResponseImplCopyWithImpl<_$TaskStatusResponseImpl>(
          this, _$identity);
}

abstract class _TaskStatusResponse implements TaskStatusResponse {
  const factory _TaskStatusResponse(
      {required final String taskId,
      required final String state,
      required final int createdAt,
      required final int updatedAt,
      required final int attempts,
      final String? errorMessage}) = _$TaskStatusResponseImpl;

  @override
  String get taskId;
  @override
  String get state;
  @override
  int get createdAt;
  @override
  int get updatedAt;
  @override
  int get attempts;
  @override
  String? get errorMessage;

  /// Create a copy of TaskStatusResponse
  /// with the given fields replaced by the non-null parameter values.
  @override
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$TaskStatusResponseImplCopyWith<_$TaskStatusResponseImpl> get copyWith =>
      throw _privateConstructorUsedError;
}
