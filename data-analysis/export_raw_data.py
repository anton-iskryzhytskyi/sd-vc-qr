import os
import json
import csv
import re
from collections import defaultdict

def extract_raw_benchmark_data(criterion_dir='../target/criterion'):
    """Extract all raw timing and size data from Criterion benchmarks"""
    
    timing_data = {}
    size_data = {}
    
    print("Extracting raw benchmark data...")
    
    for root, dirs, files in os.walk(criterion_dir):
        if 'sample.json' in files:
            sample_path = os.path.join(root, 'sample.json')
            try:
                with open(sample_path, 'r') as f:
                    sample_data = json.load(f)
                
                path_parts = root.split(os.path.sep)
                
                group_name = None
                bench_name = None
                
                for i, part in enumerate(path_parts):
                    if part in ['sd_jwt_vc_issuance', 'sd_jwt_vc_verification', 'sd_jwt_vc_sizes']:
                        group_name = part
                        if i+2 < len(path_parts) and path_parts[i+2] != 'new':
                            operation = path_parts[i+1]
                            config = path_parts[i+2]
                            bench_name = f"{operation}/{config}"
                            break
                
                if group_name and bench_name:
                    full_key = f"{group_name}_{bench_name}".replace('/', '_')
                    
                    if isinstance(sample_data, dict) and 'times' in sample_data and 'iters' in sample_data:
                        times = sample_data['times']
                        iters = sample_data['iters']
                        
                        per_op_times = []
                        for time, iter_count in zip(times, iters):
                            if iter_count > 0:
                                per_op_times.append(float(time) / float(iter_count))
                        
                        timing_data[full_key] = {
                            'group': group_name,
                            'operation': operation if 'operation' in locals() else 'unknown',
                            'config': config if 'config' in locals() else 'unknown',
                            'raw_times_ns': times,
                            'iterations': iters,
                            'per_op_times_ns': per_op_times
                        }
                        
                        print(f"  - Found timing data: {full_key} ({len(times)} samples)")
                    
            except Exception as e:
                print(f"Warning: Could not parse {sample_path}: {e}")
        
        if 'estimates.json' in files:
            estimates_path = os.path.join(root, 'estimates.json')
            try:
                with open(estimates_path, 'r') as f:
                    estimates = json.load(f)
                
                path_parts = root.split(os.path.sep)
                group_name = None
                bench_name = None
                
                for i, part in enumerate(path_parts):
                    if part in ['sd_jwt_vc_issuance', 'sd_jwt_vc_verification', 'sd_jwt_vc_sizes']:
                        group_name = part
                        if i+2 < len(path_parts) and path_parts[i+2] != 'new':
                            operation = path_parts[i+1]
                            config = path_parts[i+2]
                            bench_name = f"{operation}/{config}"
                            break
                
                if group_name and bench_name:
                    full_key = f"{group_name}_{bench_name}".replace('/', '_')
                    
                    if full_key in timing_data:
                        if 'mean' in estimates and 'point_estimate' in estimates['mean']:
                            timing_data[full_key]['mean_ns'] = estimates['mean']['point_estimate']
                        if 'std_dev' in estimates and 'point_estimate' in estimates['std_dev']:
                            timing_data[full_key]['std_dev_ns'] = estimates['std_dev']['point_estimate']
                        if 'median' in estimates and 'point_estimate' in estimates['median']:
                            timing_data[full_key]['median_ns'] = estimates['median']['point_estimate']
                            
            except Exception as e:
                print(f"Warning: Could not parse {estimates_path}: {e}")
    
    print("\\nExtracting size data from JSON...")
    
    size_json_path = f'{criterion_dir}/benchmark_size_data.json'
    if os.path.exists(size_json_path):
        try:
            with open(size_json_path, 'r') as f:
                json_size_data = json.load(f)
            
            print(f"  - Found size data in {size_json_path}")
            
            for config_name, data in json_size_data.items():
                algorithm = data.get('algorithm', '')
                field_size = data.get('field_size', '')
                credential_size_bytes = data.get('credential_size_bytes', 0)
                field_count = data.get('field_count', 0)
                disclosures_size_bytes = data.get('disclosures_size_bytes', 0)
                
                wallet_type = algorithm.replace(' ', '')
                
                size_data[config_name] = {
                    'wallet_type': wallet_type,
                    'algorithm': algorithm,
                    'field_size': field_size,
                    'jwt_size_bytes': credential_size_bytes,
                    'field_count_actual': field_count,
                    'disclosures_size_bytes': disclosures_size_bytes
                }
                
                print(f"    - {config_name}: JWT={credential_size_bytes} bytes, Disclosures={disclosures_size_bytes} bytes")
                
        except Exception as e:
            print(f"Warning: Could not parse {size_json_path}: {e}")
    
    return timing_data, size_data

def parse_size_string(size_str):
    if not size_str or size_str.strip() == '-':
        return 0
    
    match = re.search(r'(\d+(?:\.\d+)?)', size_str)
    if not match:
        return 0
    
    value = float(match.group(1))
    
    if 'KB' in size_str.upper():
        return int(value * 1024)
    elif 'MB' in size_str.upper():
        return int(value * 1024 * 1024)
    elif 'GB' in size_str.upper():
        return int(value * 1024 * 1024 * 1024)
    else:
        return int(value)

def export_to_csv(timing_data, size_data, output_dir='../target/criterion'):
    if not os.path.exists(output_dir):
        os.makedirs(output_dir)
    
    print(f"\\nExporting data to {output_dir}/...")
    
    timing_summary_path = os.path.join(output_dir, 'timing_summary.csv')
    with open(timing_summary_path, 'w', newline='') as f:
        writer = csv.writer(f)
        writer.writerow([
            'benchmark_key', 'group', 'operation', 'config', 'wallet_type', 'field_count', 'field_size',
            'sample_count', 'mean_ns', 'median_ns', 'std_dev_ns', 'min_ns', 'max_ns',
            'mean_ms', 'median_ms', 'std_dev_ms', 'min_ms', 'max_ms'
        ])
        
        for key, data in timing_data.items():
            config_match = re.search(r'([A-Za-z0-9+]+)_(\d+)_fields_(\d+)_bytes', data['config'])
            if config_match:
                wallet_type, field_count, field_size = config_match.groups()
            else:
                wallet_type = data['config'].split('_')[0] if '_' in data['config'] else data['config']
                field_count, field_size = 0, 0
            
            per_op_times = data.get('per_op_times_ns', [])
            if per_op_times:
                min_ns = min(per_op_times)
                max_ns = max(per_op_times)
                mean_ns = data.get('mean_ns', sum(per_op_times) / len(per_op_times))
                median_ns = data.get('median_ns', sorted(per_op_times)[len(per_op_times)//2])
                std_dev_ns = data.get('std_dev_ns', 0)
                
                writer.writerow([
                    key, data['group'], data['operation'], data['config'], wallet_type,
                    int(field_count), int(field_size), len(per_op_times),
                    mean_ns, median_ns, std_dev_ns, min_ns, max_ns,
                    mean_ns / 1_000_000, median_ns / 1_000_000, std_dev_ns / 1_000_000,
                    min_ns / 1_000_000, max_ns / 1_000_000
                ])
    
    print(f"  - Exported timing summary to {timing_summary_path}")
    
    raw_samples_path = os.path.join(output_dir, 'raw_timing_samples.csv')
    with open(raw_samples_path, 'w', newline='') as f:
        writer = csv.writer(f)
        writer.writerow([
            'benchmark_key', 'group', 'operation', 'config', 'wallet_type', 'field_count', 'field_size',
            'sample_index', 'raw_time_ns', 'iterations', 'per_op_time_ns', 'per_op_time_ms'
        ])
        
        for key, data in timing_data.items():
            config_match = re.search(r'([A-Za-z0-9+]+)_(\d+)_fields_(\d+)_bytes', data['config'])
            if config_match:
                wallet_type, field_count, field_size = config_match.groups()
            else:
                wallet_type = data['config'].split('_')[0] if '_' in data['config'] else data['config']
                field_count, field_size = 0, 0
            
            raw_times = data.get('raw_times_ns', [])
            iterations = data.get('iterations', [])
            per_op_times = data.get('per_op_times_ns', [])
            
            for i, (raw_time, iter_count, per_op_time) in enumerate(zip(raw_times, iterations, per_op_times)):
                writer.writerow([
                    key, data['group'], data['operation'], data['config'], wallet_type,
                    int(field_count), int(field_size), i, raw_time, iter_count,
                    per_op_time, per_op_time / 1_000_000
                ])
    
    print(f"  - Exported raw timing samples to {raw_samples_path}")
    
    if size_data:
        size_path = os.path.join(output_dir, 'size_data.csv')
        with open(size_path, 'w', newline='') as f:
            writer = csv.writer(f)
            writer.writerow([
                'config_name', 'wallet_type', 'field_count', 'field_size',
                'jwt_size_bytes', 'jwt_size_str', 'field_count_actual',
                'disclosures_size_bytes', 'disclosures_size_str', 'total_size_bytes'
            ])
            
            for config_name, data in size_data.items():
                total_size = data['jwt_size_bytes'] + data['disclosures_size_bytes']
                writer.writerow([
                    config_name, data['wallet_type'], data.get('field_count', 0), data.get('field_size', ''),
                    data['jwt_size_bytes'], data.get('jwt_size_str', ''), data['field_count_actual'],
                    data['disclosures_size_bytes'], data.get('disclosures_size_str', ''), total_size
                ])
        
        print(f"  - Exported size data to {size_path}")
    
    combined_path = os.path.join(output_dir, 'combined_summary.csv')
    with open(combined_path, 'w', newline='') as f:
        writer = csv.writer(f)
        writer.writerow([
            'wallet_type', 'field_count', 'field_size', 'operation',
            'mean_time_ms', 'median_time_ms', 'std_dev_time_ms', 'sample_count',
            'jwt_size_bytes', 'disclosures_size_bytes', 'total_size_bytes'
        ])
        
        timing_by_config = defaultdict(dict)
        for key, data in timing_data.items():
            config_match = re.search(r'([A-Za-z0-9+]+)_(\d+)_fields_(\d+)_bytes', data['config'])
            if config_match:
                wallet_type, field_count, field_size = config_match.groups()
                config_key = f"{wallet_type}_{field_count}_fields_{field_size}_bytes"
                timing_by_config[config_key][data['operation']] = data
        
        for config_name, operations in timing_by_config.items():
            size_info = size_data.get(config_name, {})
            
            for operation, timing_info in operations.items():
                config_match = re.search(r'([A-Za-z0-9+]+)_(\d+)_fields_(\d+)_bytes', config_name)
                if config_match:
                    wallet_type, field_count, field_size = config_match.groups()
                    
                    per_op_times = timing_info.get('per_op_times_ns', [])
                    if per_op_times:
                        mean_ms = (timing_info.get('mean_ns', sum(per_op_times) / len(per_op_times))) / 1_000_000
                        median_ms = (timing_info.get('median_ns', sorted(per_op_times)[len(per_op_times)//2])) / 1_000_000
                        std_dev_ms = timing_info.get('std_dev_ns', 0) / 1_000_000
                        
                        writer.writerow([
                            wallet_type, int(field_count), int(field_size), operation,
                            mean_ms, median_ms, std_dev_ms, len(per_op_times),
                            size_info.get('jwt_size_bytes', 0),
                            size_info.get('disclosures_size_bytes', 0),
                            size_info.get('jwt_size_bytes', 0) + size_info.get('disclosures_size_bytes', 0)
                        ])
    
    print(f"  - Exported combined summary to {combined_path}")

def main():
    print("SD-JWT VC Benchmark Data Extractor")
    print("==================================\\n")
    
    import os
    if os.path.exists('./target/criterion'):
        criterion_dir = './target/criterion'
        output_dir = './target/criterion'
    else:
        criterion_dir = '../target/criterion'
        output_dir = '../target/criterion'
    
    print(f"Using criterion_dir: {criterion_dir}")
    print(f"Using output_dir: {output_dir}")
    
    timing_data, size_data = extract_raw_benchmark_data(criterion_dir)
    
    print(f"\\nExtracted:")
    print(f"  - {len(timing_data)} timing benchmarks")
    print(f"  - {len(size_data)} size measurements")
    
    if timing_data or size_data:
        export_to_csv(timing_data, size_data, output_dir)
        
        print("\\nData export completed!")
        print("\\nGenerated files:")
        print("  - timing_summary.csv: Statistical summary of all timing data")
        print("  - raw_timing_samples.csv: Individual timing measurements")
        print("  - size_data.csv: JWT and disclosure size measurements")
        print("  - combined_summary.csv: Combined timing and size data")
    else:
        print("\\nNo benchmark data found. Make sure to run 'cargo bench' first!")

if __name__ == "__main__":
    main()